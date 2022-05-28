use anyhow::Result;
use clap::{ArgMatches, Command};

pub(crate) fn cli() -> Vec<Command<'static>> {
    vec![dump::cli(), header::cli(), schema::cli()]
}

pub(crate) async fn dispatch(matches: ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("dump", args)) => dump::exec(args).await?,
        Some(("header", args)) => header::exec(args).await?,
        Some(("schema", args)) => schema::exec(args).await?,
        _ => unreachable!(),
    }
    std::process::exit(0)
}

mod dump;
mod header;
mod schema;
