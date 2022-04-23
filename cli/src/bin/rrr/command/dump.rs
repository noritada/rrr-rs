use crate::common::read_from_source;
use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use rrr::JsonDisplay;

pub(crate) fn cli() -> Command<'static> {
    Command::new("dump")
        .about("Dump the data of the specified file")
        .arg(Arg::new("file").required(true))
}

pub(crate) async fn exec(args: &ArgMatches) -> Result<()> {
    let fname = args.value_of("file").unwrap();
    let (schema, body_buf) = read_from_source(fname, true).await?;

    println!("{}", JsonDisplay::new(&schema, &body_buf));

    Ok(())
}
