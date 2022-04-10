mod command;
mod common;
mod visitor;

use anyhow::{anyhow, Result};
use clap::{crate_name, crate_version, Command};
use rrr::{DataReader, Schema};

fn app() -> Command<'static> {
    Command::new(crate_name!())
        .version(crate_version!())
        .arg_required_else_help(true)
        .subcommands(command::cli())
}

pub(crate) fn read_from_file(fname: &str) -> Result<(Schema, Vec<u8>)> {
    let input_path = std::path::PathBuf::from(fname);
    let f = std::fs::File::open(input_path)?;
    let f = std::io::BufReader::new(f);
    let mut f = DataReader::new(f);
    f.read().map_err(|e| anyhow!(e))
}

fn main() -> Result<()> {
    let matches = app().get_matches();
    command::dispatch(matches)
}
