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
async fn main() {
    if let Err(err) = try_main().await {
        let red = console::Style::new().red();
        eprintln!("{}: {}", red.apply_to("error"), err);
        std::process::exit(1);
    }
}

async fn try_main() -> Result<()> {
    let matches = app().get_matches();
    command::dispatch(matches).await
}
