use std::str::FromStr;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{prelude::*, stdout};
use std::path::PathBuf;

use clap::{App, Arg, SubCommand, Shell};

use rayon::prelude::*;

use crate::error::*;
use crate::render;
use crate::spec::TemplateSpec;

pub(crate) fn get_parser<'a, 'b>() -> App<'a, 'b> {
    clap::app_from_crate!()
        .subcommand(
            SubCommand::with_name("generate")
                .about("Generate a single file from TEMPLATE and DATA, print to OUTPUT.")
                .arg(Arg::with_name("TEMPLATE").required(true))
                .arg(Arg::with_name("DATA").required(true))
                .arg(Arg::with_name("OUTPUT").default_value("-")),
        )
        .subcommand(
            SubCommand::with_name("completion")
                .about("Print shell completions for ttgen, in SHELL format")
                .arg(Arg::with_name("SHELL").help("Name of shell").possible_values(&Shell::variants()).required(true))
                .arg(Arg::with_name("OUTPUT").default_value("-")),
        )
        .subcommand(
            SubCommand::with_name("clean")
                .about("Delete all output files referenced by SPEC")
                .arg(
                    Arg::with_name("SPEC")
                        .help("A ttgen-spec file describing all of the templates to clean.")
                        .required(true),
                )
                .arg(
                    Arg::with_name("JOBS")
                        .help("Maximum number of parallel jobs to run.  Default or 0 is infinite.")
                        .short("j")
                        .long("max-jobs")
                        .default_value("0"),
                ),
        )
        .subcommand(
            SubCommand::with_name("multigen")
                .about("Generate all output files using SPEC as a build reference")
                .arg(
                    Arg::with_name("SPEC")
                        .help("A ttgen-spec file describing all of the templates to clean.")
                        .required(true),
                )
                .arg(
                    Arg::with_name("FORCE")
                        .help("Remake all templates, do not check mod times.")
                        .short("f")
                        .long("force")
                        .takes_value(false)
                )
                .arg(
                    Arg::with_name("JOBS")
                        .help("Maximum number of parallel jobs to run.  Default or 0 is infinite.")
                        .short("j")
                        .long("max-jobs")
                        .default_value("0"),
                ),
        )
        .subcommand(
            SubCommand::with_name("report")
                .aliases(&["dry-run", "query"])
                .about("Analyze SPEC and report based on COMMAND")
                // .usage("ttgen report COMMAND SPEC")
                .arg(Arg::with_name("COMMAND").possible_values(&["clean", "multigen", "count"]))
                .arg(
                    Arg::with_name("SPEC")
                        .help("A ttgen-spec file describing all of the templates to examine.")
                        .required(true),
                )
                .arg(
                    Arg::with_name("FORCE")
                        .help("Do not check mod times or existence, assume operation will run.")
                        .short("f")
                        .long("force")
                        .takes_value(false)
                )
        )
}

pub fn parse_args<I, T>(a: &mut App, arg_iter: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let matches = a.get_matches_from_safe_borrow(arg_iter)?;
    match matches.subcommand() {
        ("generate", Some(args)) => generate(args),
        ("multigen", Some(args)) => multigen(args),
        ("report", Some(args)) => report(args),
        ("clean", Some(args)) => clean(args),
        ("completion", Some(args)) => completion(a, args),
        _ => unimplemented!(),
    }
}

fn box_writer(s: &str) -> Result<Box<dyn Write>> {
    let writer: Box<dyn Write> = match s {
        "-" => Box::new(stdout()),
        other => {
            let output = PathBuf::from(other);
            if let Some(p) = &output.parent() {
                fs::create_dir_all(p)?;
            };
            Box::new(File::create(output)?)
        }
    };

    Ok(writer)
}

fn completion(app: &mut App, args: &clap::ArgMatches) -> Result<()> {
    let shell: Shell = Shell::from_str(&args.value_of("SHELL").unwrap().to_ascii_uppercase()).unwrap();
    let bin_name = clap::crate_name!();
    let mut writer = box_writer(args.value_of("OUTPUT").unwrap())?;
    app.gen_completions_to(bin_name, shell, &mut writer);
    Ok(())
}

fn clean(args: &clap::ArgMatches) -> Result<()> {
    let spec_file = args.value_of("SPEC").unwrap();
    let specs: Vec<TemplateSpec> = serde_json::from_reader(File::open(spec_file)?)?;
    
    specs.par_iter().map(|s| &s.output).for_each(|p| {
        if let Err(e) = fs::remove_file(p) {
            eprintln!("failed to remove: {}: error: {}", p.display(), e);
        } else {
            println!("removed: {}", p.display());
        }
    });

    Ok(())
}

fn generate(args: &clap::ArgMatches) -> Result<()> {
    // Unwrap due to parser guarantees.
    let data = args.value_of("DATA").unwrap();
    let template = args.value_of("TEMPLATE").unwrap();
    let output = args.value_of("OUTPUT").unwrap();
    let mut out_writer = box_writer(output)?;
    let spec = TemplateSpec::new("Anonymous", data, template, output)?;
    let hb = render::get_renderer();
    render::render_with_writer(&spec, &hb, &mut out_writer)
}

fn multigen(args: &clap::ArgMatches) -> Result<()> {
    let spec_file = args.value_of("SPEC").unwrap();
    let specs: Vec<TemplateSpec> = serde_json::from_reader(File::open(spec_file)?)?;
    let hb = render::get_renderer();

    let force = args.is_present("FORCE");

    specs
        .par_iter()
        .filter_map(|s: &TemplateSpec| {
            if force || s.should_build() {
                Some((render::render_with(&s, &hb), s))
            } else {
                println!("skipped: {}", &s.name);
                None
            }
        })
        .for_each(|(r, s)| match r {
            Err(e) => {
                eprintln!("error: {}: {}", s.name, e);
            }
            Ok(_) => {
                println!("success: {}", s.name);
            }
        });
    Ok(())
}

fn report(args: &clap::ArgMatches) -> Result<()> {
    let spec_file = args.value_of("SPEC").unwrap();
    let specs: Vec<TemplateSpec> = serde_json::from_reader(File::open(spec_file)?)?;
    let force = args.is_present("FORCE");
    let command = args.value_of("COMMAND").unwrap();
    
    match command {
        "clean" => {
            specs.par_iter().map(|s| &s.output).for_each(|p| {
                if p.exists() {
                    println!("Would remove: {}", p.display());
                }
            });
        },
        "multigen" => {
            specs.par_iter().for_each(|s| {
                if force || s.should_build() {
                    println!("Would build: {}", s.output.display());
                } else {
                    println!("Would skip: {}", s.output.display());                    
                }
            })
        },
        "count" => {
            println!("{}", specs.len());
        },
        _ => unreachable!()
    };

    Ok(())
}