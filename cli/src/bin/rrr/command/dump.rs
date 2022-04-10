use crate::read_from_file;
use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use rrr::{JsonDisplay, Schema};

pub(crate) fn cli() -> Command<'static> {
    Command::new("dump")
        .about("Dump the data of the specified file")
        .arg(Arg::new("file").required(true))
}

pub(crate) fn exec(args: &ArgMatches) -> Result<()> {
    let fname = args.value_of("file").unwrap();
    let (schema, body_buf) = read_from_file(fname)?;

    println!("{}", JsonDisplay::new(&schema, &body_buf));

    Ok(())
}
