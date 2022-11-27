use anyhow::Result;
use clap::{arg, ArgAction, ArgMatches, Command};
use clap_complete::{generate, Generator, Shell};

pub(crate) fn cli() -> Command {
    Command::new("completions")
        .about("Generate shell completions for your shell to stdout")
        .arg(
            arg!(<SHELL> "The shell to generate completions for")
                .action(ArgAction::Set)
                .value_parser(clap::value_parser!(Shell)),
        )
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut std::io::stdout());
}

pub(crate) async fn exec(args: &ArgMatches) -> Result<()> {
    let generator = args.get_one::<Shell>("SHELL").copied().unwrap();
    let mut cmd = crate::app();
    print_completions(generator, &mut cmd);

    Ok(())
}
