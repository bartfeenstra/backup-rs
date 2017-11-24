extern crate argparse;
extern crate backups;

use argparse::{ArgumentParser, List, Store, StoreTrue, Print};
use backups::{Configuration, Environment, Runner};
use backups::errors::*;
use std::io::{stdout, stderr};
use std::path::Path;
use std::result::Result as StdResult;
use std::str::FromStr;
use std::ffi::OsString;

#[derive(Debug)]
enum Command {
    None,
    Backup,
    Restore,
}

impl FromStr for Command {
    type Err = ();
    fn from_str(name: &str) -> StdResult<Command, ()> {
        match name {
            "backup" => Ok(Command::Backup),
            "restore" => Ok(Command::Restore),
            _ => Ok(Command::None),
        }
    }
}

fn backup(args: Vec<String>) -> Result<()> {
    let mut configuration_file_path = String::new();
    let mut verbose = false;
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Back up changes since the last back-up.");
        ap.refer(&mut verbose)
            .add_option(&["-v", "--verbose"], StoreTrue, "Output additional information.");
        ap.refer(&mut configuration_file_path)
            .add_option(&["-c", "--configuration"], Store, "The path to the configuration file")
            .required();
        // @todo How to handle these errors?
        ap.parse(args, &mut stdout(), &mut stderr());
    }
    let configuration_file_path = OsString::from(configuration_file_path);
    let mut configuration = Configuration::from_file(Path::new(&configuration_file_path))?;
    if verbose {
        configuration.verbose = true;
    }
    let environment = Environment::new(configuration);
    let mut runner = Runner::new(environment);
    runner.backup()
}

fn restore(args: Vec<String>) -> Result<()> {
    let mut ap = ArgumentParser::new();
    ap.set_description("Restores the latest back-up.");
    ap.parse_args_or_exit();
    // @todo Should this command take a path to restore just for safety? What if the entire system is backed up? User's own responsibility?
    // @todo It should take at least an optional path for easy recovery of single files or dirs.
    // @todo Also add user confirmation, because this overrides live data. Ternary: NO, YES-remove-nonexistent-files, YES-keep-nonexitent-files.
    println!("Finish this.");
    Ok(())
}

fn main() {
    let mut subcommand = Command::None;
    let mut args = vec!();
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("This program lets you back up files using rsync.");
        ap.refer(&mut subcommand)
            .add_argument("command", Store, r#"The back-up command to execute."#);
        ap.refer(&mut args)
            .add_argument("arguments", List,
                r#"Arguments for command"#);
        ap.add_option(&["-V", "--version"],
                      Print(env!("CARGO_PKG_VERSION").to_string()), "Show version");
        ap.stop_on_first_argument(true);
        ap.parse_args_or_exit();
    }
    args.insert(0, format!("{:?}", subcommand));
    let result = match subcommand {
        Command::Backup => backup(args),
        Command::Restore => restore(args),
        Command::None => {
            println!("Use --help for usage information.");
            Ok(())
        },
    };
    match result {
        Ok(_) => println!("Done."),
        Err(e) => {
            println!("Error: {}", e);
            for e in e.iter().skip(1) {
                println!("Caused by: {}", e);
            }
        },
    }
//    if let Err(ref e) = run_main(configuration_file_path, verbose) {
//        println!("Error: {}", e);
//
//        for e in e.iter().skip(1) {
//            println!("Caused by: {}", e);
//        }
//
//        // The backtrace is not always generated. Try to run this example
//        // with `RUST_BACKTRACE=1`.
//        if let Some(backtrace) = e.backtrace() {
//            println!("Backtrace: {:?}", backtrace);
//        }
//
//        ::std::process::exit(1);
//    }
}
