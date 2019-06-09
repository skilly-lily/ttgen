use std::ffi::OsString;
use std::fs::File;

use clap::{App, Arg, SubCommand};

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
                .arg(Arg::with_name("SHELL"))
                .arg(Arg::with_name("OUTPUT")),
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
                    Arg::with_name("JOBS")
                        .help("Maximum number of parallel jobs to run.  Default or 0 is infinite.")
                        .short("j")
                        .long("max-jobs")
                        .default_value("0"),
                ),
        )
        .subcommand(
            SubCommand::with_name("report")
                .alias("dry-run")
                .about("Analyze SPEC and report based on COMMAND")
                // .usage("ttgen report COMMAND SPEC")
                .arg(Arg::with_name("COMMAND").possible_values(&["clean", "multigen", "count"]))
                .arg(Arg::with_name("SPEC")),
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
        _ => unimplemented!(),
    }
}

fn generate(args: &clap::ArgMatches) -> Result<()> {
    // Unwrap due to parser guarantees.
    let data = args.value_of("DATA").unwrap();
    let template = args.value_of("TEMPLATE").unwrap();
    let output = match args.value_of("OUTPUT").unwrap() {
        // unwrap guaranteed because of default
        "-" => None,
        x => Some(x),
    };
    let spec = TemplateSpec::new("Anonymous", data, template, output)?;
    let hb = render::get_renderer();
    render::render_with(&spec, &hb)
}

fn multigen(args: &clap::ArgMatches) -> Result<()> {
    let spec_file = args.value_of("SPEC").unwrap();
    let specs: Vec<TemplateSpec> = serde_json::from_reader(File::open(spec_file)?)?;
    let hb = render::get_renderer();
    specs
        .par_iter()
        .filter_map(|s: &TemplateSpec| {
            if s.should_build() {
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
