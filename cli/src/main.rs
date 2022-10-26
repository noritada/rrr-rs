mod command;
mod common;
mod diagnostics;
mod visitor;

use anyhow::Result;
use clap::{crate_name, crate_version, Command};

fn app() -> Command {
    Command::new(crate_name!())
        .version(crate_version!())
        .arg_required_else_help(true)
        .subcommands(command::cli())
}

#[tokio::main]
async fn main() {
    if let Err(err) = try_main().await {
        let red = console::Style::new().red();
        eprintln!("{}: {err}", red.apply_to("error"));
        std::process::exit(1);
    }
}

async fn try_main() -> Result<()> {
    let matches = app().get_matches();
    command::dispatch(matches).await
}
