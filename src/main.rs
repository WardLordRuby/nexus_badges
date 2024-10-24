use crate::{
    await_user_for_end,
    commands::{init_actions, init_remote, process, update_args, Modify},
    models::cli::{Cli, Commands},
};
use clap::Parser;
use nexus_badges::*;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let input_mods = match startup(cli.remote) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("{err}");
            await_user_for_end();
            return;
        }
    };

    if let Some(command) = cli.command {
        match command {
            Commands::SetArg(args) => update_args(input_mods, args)
                .await
                .unwrap_or_else(|err| eprintln!("{err}")),
            Commands::Add(details) => input_mods
                .add_mod(details)
                .unwrap_or_else(|err| eprintln!("{err}")),
            Commands::Remove(details) => input_mods
                .remove_mod(details)
                .unwrap_or_else(|err| eprintln!("{err}")),
            Commands::Init => init_remote(input_mods)
                .await
                .unwrap_or_else(|err| eprintln!("{err}")),
            Commands::InitActions => init_actions(input_mods)
                .await
                .unwrap_or_else(|err| eprintln!("{err}")),
        }
    } else {
        process(input_mods)
            .await
            .unwrap_or_else(|err| eprintln!("{err}"));
        await_user_for_end();
    }
}
