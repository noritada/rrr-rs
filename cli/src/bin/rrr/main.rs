mod command;
mod common;
mod error;
mod visitor;

use anyhow::Result;
use clap::{crate_name, crate_version, Command};

fn app() -> Command<'static> {
    Command::new(crate_name!())
        .version(crate_version!())
        .arg_required_else_help(true)
        .subcommands(command::cli())
}

#[tokio::main]
async fn main() -> Result<()> {
    let matches = app().get_matches();
    command::dispatch(matches).await
}
