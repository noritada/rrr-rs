use crate::common::read_from_source;
use anyhow::Result;
use clap::{arg, ArgAction, ArgMatches, Command};
use rrr::{DataReaderOptions, JsonDisplay};

pub(crate) fn cli() -> Command {
    Command::new("dump")
        .about("Dump the data of the specified file")
        .arg(
            arg!(--"ignore-size" r#"Ignore the value of "data_size" field in reading"#)
                .action(ArgAction::SetTrue),
        )
        .arg(arg!(<PATH_OR_URI> "Path or S3 URI of the file").required(true))
}

pub(crate) async fn exec(args: &ArgMatches) -> Result<()> {
    let fname = args.get_one::<String>("PATH_OR_URI").unwrap();
    let options = DataReaderOptions::ENABLE_READING_BODY;
    let options = if args.get_flag("ignore-size") {
        options.union(DataReaderOptions::IGNORE_DATA_SIZE_FIELD)
    } else {
        options
    };
    let (schema, _, body_buf) = read_from_source(fname, None, options).await?;

    println!("{}", JsonDisplay::new(&schema, &body_buf));

    Ok(())
}
