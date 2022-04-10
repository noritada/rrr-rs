use anyhow::Result;
use clap::{ArgMatches, Command};

pub(crate) fn cli() -> Vec<Command<'static>> {
    vec![dump::cli(), schema::cli()]
}

pub(crate) fn dispatch(matches: ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("dump", args)) => dump::exec(args),
        Some(("schema", args)) => schema::exec(args),
        _ => unreachable!(),
    }
}

mod dump;
mod schema;
