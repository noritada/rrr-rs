use crate::read_from_file;
use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use rrr::{Schema, SchemaOnelineDisplay, SchemaTreeDisplay};

pub(crate) fn cli() -> Command<'static> {
    Command::new("schema")
        .about("Display the schema of the specified file")
        .arg(
            Arg::new("tree")
                .help("Display in the tree format")
                .short('t')
                .long("tree"),
        )
        .arg(Arg::new("file").required(true))
}

pub(crate) fn exec(args: &ArgMatches) -> Result<()> {
    let fname = args.value_of("file").unwrap();
    let buf = read_from_file(fname)?;

    let schema: Schema = buf.as_slice().try_into()?;
    if args.is_present("tree") {
        print!("{}", SchemaTreeDisplay(&schema.ast))
    } else {
        println!("{}", SchemaOnelineDisplay(&schema.ast))
    }

    Ok(())
}
