use anyhow::Result;
use clap::{ArgMatches, Command};

pub(crate) fn cli() -> Vec<Command<'static>> {
    vec![schema::cli()]
}

pub(crate) fn dispatch(matches: ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("schema", args)) => schema::exec(args),
        _ => unreachable!(),
    }
}

mod schema;
