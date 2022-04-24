use crate::{
    common::read_from_source,
    visitor::{FieldCounter, SchemaTreeDisplay},
};
use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use console::Term;
use rrr::SchemaOnelineDisplay;

pub(crate) fn cli() -> Command<'static> {
    Command::new("schema")
        .about("Display the schema of the specified file")
        .arg(
            Arg::new("tree")
                .help("Display in the tree format")
                .short('t')
                .long("tree"),
        )
        .arg(
            Arg::new("PATH_OR_URI")
                .help("Path or S3 URI of the file")
                .required(true),
        )
}

pub(crate) async fn exec(args: &ArgMatches) -> Result<()> {
    let fname = args.value_of("PATH_OR_URI").unwrap();
    let (schema, _) = read_from_source(fname, false).await?;

    if args.is_present("tree") {
        let user_attended = console::user_attended();

        let term = Term::stdout();
        let (height, _width) = term.size();
        let num_lines = FieldCounter::count(&schema.ast)?;
        if num_lines > height.into() {
            crate::common::start_pager();
        }

        if user_attended {
            console::set_colors_enabled(true);
        }

        print!("{}", SchemaTreeDisplay(&schema.ast))
    } else {
        println!("{}", SchemaOnelineDisplay(&schema.ast))
    }

    Ok(())
}
