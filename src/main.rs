use clap::Parser;
use nexus_badges::{
    await_user_for_end,
    commands::{
        init_actions, init_remote, process, update_args, update_cache_key, version, Modify,
    },
    models::cli::{Cli, Commands},
    services::git::set_workflow_state,
    startup, unsupported,
};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Some(ref command) = cli.command {
        match command {
            Commands::Version => {
                version(cli.remote)
                    .await
                    .unwrap_or_else(|err| eprintln!("{err}"));
                return;
            }
            &Commands::Automation { state } => {
                unsupported!(command, on_remote, cli.remote);
                set_workflow_state(state)
                    .await
                    .unwrap_or_else(|err| eprintln!("{err}"));
                return;
            }
            Commands::UpdateCacheKey { old, new } => {
                unsupported!(command, on_local, cli.remote);
                update_cache_key(old, new).await.unwrap_or_else(|err| {
                    eprintln!("{err}");
                    std::process::exit(1)
                });
                return;
            }
            _ => (),
        }
    }

    let input_mods = match startup(cli.remote) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("{err}");
            if cli.remote {
                std::process::exit(1);
            }
            await_user_for_end(cli.remote);
            return;
        }
    };

    if let Some(command) = cli.command {
        unsupported!(command, on_remote, cli.remote);
        match command {
            Commands::SetArg(args) => update_args(input_mods, args)
                .await
                .unwrap_or_else(|err| eprintln!("{err}")),
            Commands::Add(details) => input_mods
                .add_mod(details)
                .await
                .unwrap_or_else(|err| eprintln!("{err}")),
            Commands::Remove(details) => input_mods
                .remove_mod(details)
                .await
                .unwrap_or_else(|err| eprintln!("{err}")),
            Commands::Init => init_remote(input_mods)
                .await
                .unwrap_or_else(|err| eprintln!("{err}")),
            Commands::InitActions => init_actions(input_mods)
                .await
                .unwrap_or_else(|err| eprintln!("{err}")),
            Commands::UpdateCacheKey { old: _, new: _ } => unreachable!("by repo-variable guard"),
            Commands::Automation { state: _ } => unreachable!("by automation guard"),
            Commands::Version => unreachable!("by version guard"),
        }
    } else {
        process(input_mods, cli.remote).await.unwrap_or_else(|err| {
            eprintln!("{err}");
            if cli.remote {
                std::process::exit(1);
            }
        });
        await_user_for_end(cli.remote);
    }
}
