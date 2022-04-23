use anyhow::Result;
use clap::{ArgMatches, Command};

pub(crate) fn cli() -> Vec<Command<'static>> {
    vec![dump::cli(), schema::cli()]
}

pub(crate) async fn dispatch(matches: ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("dump", args)) => dump::exec(args).await,
        Some(("schema", args)) => schema::exec(args).await,
        _ => unreachable!(),
    }
}

mod dump;
mod schema;
