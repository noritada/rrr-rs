use crate::common::read_from_source;
use anyhow::Result;
use clap::{arg, command, ArgMatches, Command};
use rrr::JsonDisplay;

pub(crate) fn cli() -> Command<'static> {
    command!("dump")
        .about("Dump the data of the specified file")
        .arg(arg!(<PATH_OR_URI> "Path or S3 URI of the file").required(true))
}

pub(crate) async fn exec(args: &ArgMatches) -> Result<()> {
    let fname = args.get_one::<String>("PATH_OR_URI").unwrap();
    let (schema, _, body_buf) = read_from_source(fname, true, None).await?;

    println!("{}", JsonDisplay::new(&schema, &body_buf));

    Ok(())
}
