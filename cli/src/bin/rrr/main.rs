mod command;

use anyhow::Result;
use clap::{crate_name, crate_version, Command};
use std::io::Read;

fn app() -> Command<'static> {
    Command::new(crate_name!())
        .version(crate_version!())
        .arg_required_else_help(true)
        .subcommands(command::cli())
}

pub(crate) fn read_from_file(fname: &str) -> std::io::Result<Vec<u8>> {
    let input_path = std::path::PathBuf::from(fname);
    let f = std::fs::File::open(input_path)?;
    let mut f = std::io::BufReader::new(f);
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;

    Ok(buf)
}

fn main() -> Result<()> {
    let matches = app().get_matches();
    command::dispatch(matches)
}