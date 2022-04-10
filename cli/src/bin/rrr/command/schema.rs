use crate::{
    read_from_file,
    visitor::{FieldCounter, SchemaTreeDisplay},
};
use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use console::Term;
use rrr::{Schema, SchemaOnelineDisplay};

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
