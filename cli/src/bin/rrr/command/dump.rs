use crate::read_from_file;
use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use rrr::{JsonDisplay, Schema};

pub(crate) fn cli() -> Command<'static> {
    Command::new("dump")
        .about("Dump the data of the specified file")
        .arg(Arg::new("schema").required(true))
        .arg(Arg::new("body").required(true))
}

pub(crate) fn exec(args: &ArgMatches) -> Result<()> {
    let schema_fname = args.value_of("schema").unwrap();
    let schema_buf = read_from_file(schema_fname)?;
    let schema: Schema = schema_buf.as_slice().try_into()?;

    let body_fname = args.value_of("body").unwrap();
    let body_buf = read_from_file(body_fname)?;

    println!("{}", JsonDisplay::new(&schema, &body_buf));

    Ok(())
}
