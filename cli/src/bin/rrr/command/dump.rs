use crate::common::read_from_source;
use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use rrr::JsonDisplay;

pub(crate) fn cli() -> Command<'static> {
    Command::new("dump")
        .about("Dump the data of the specified file")
        .arg(
            Arg::new("PATH_OR_URI")
                .help("Path or S3 URI of the file")
                .required(true),
        )
}

pub(crate) async fn exec(args: &ArgMatches) -> Result<()> {
    let fname = args.value_of("PATH_OR_URI").unwrap();
    let (schema, body_buf) = read_from_source(fname, true, None).await?;

    println!("{}", JsonDisplay::new(&schema, &body_buf));

    Ok(())
}
