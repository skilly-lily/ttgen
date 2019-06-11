#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

#[macro_use]
extern crate clap;

use std::env::args_os;
use std::fmt::Display;

mod cli;
mod error;
mod render;
mod spec;

fn exit<D: Display>(msg: D, exitcode: i32) -> ! {
    if exitcode == 0 {
        println!("{}", msg);
    } else {
        eprintln!("{}", msg);
    };

    std::process::exit(exitcode);
}

fn main() {
    let mut app = cli::get_parser();
    if let Err(e) = cli::parse_args(&mut app, args_os()) {
        exit(e, 1);
    };
}
