use anyhow::Result;
use clap::{arg, ArgAction, ArgMatches, Command};
use console::Term;
use rrr::{DataReaderOptions, SchemaOnelineDisplay};

use crate::{
    common::read_from_source,
    visitor::{FieldCounter, SchemaTreeDisplay},
};

pub(crate) fn cli() -> Command {
    Command::new("schema")
        .about("Display the schema of the specified file")
        .arg(arg!(-t --tree "Display in the tree format").action(ArgAction::SetTrue))
        .arg(
            arg!(N: -b --bytes <N> "Read only the first N bytes from the S3 bucket")
                .default_value("4096")
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(arg!(<PATH_OR_URI> "Path or S3 URI of the file").required(true))
}

pub(crate) async fn exec(args: &ArgMatches) -> Result<()> {
    let fname = args.get_one::<String>("PATH_OR_URI").unwrap();
    let n_bytes = args.get_one::<usize>("N").unwrap();
    let options = DataReaderOptions::ALLOW_TRAILING_COMMA
        | DataReaderOptions::ALLOW_EMPTY_FIELD_NAME
        | DataReaderOptions::ALLOW_STR_INSTEAD_OF_NSTR;
    let (schema, _, _) = read_from_source(fname, Some(n_bytes), options).await?;

    if args.get_flag("tree") {
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
