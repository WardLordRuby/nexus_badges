use crate::{
    await_user_for_end,
    cli::{Cli, Commands},
};
use clap::Parser;
use nexus_badges::*;

#[tokio::main]
async fn main() {
    let input = match startup() {
        Ok(data) => data,
        Err(err) => {
            eprintln!("{err}");
            await_user_for_end();
            return;
        }
    };

    let cli = Cli::parse();

    if let Some(command) = cli.command {
        match command {
            Commands::SetArg(args) => input
                .update_args(args)
                .await
                .unwrap_or_else(|err| eprintln!("{err}")),
            Commands::Init => init_remote(input)
                .await
                .unwrap_or_else(|err| eprintln!("{err}")),
            Commands::Add(details) => input
                .add_mod(details)
                .unwrap_or_else(|err| eprintln!("{err}")),
            Commands::Remove(details) => input
                .remove_mod(details)
                .unwrap_or_else(|err| eprintln!("{err}")),
            Commands::InitActions => init_actions(input)
                .await
                .unwrap_or_else(|err| eprintln!("{err}")),
        }
    } else {
        process(input)
            .await
            .unwrap_or_else(|err| eprintln!("{err}"));
        await_user_for_end();
    }
}
