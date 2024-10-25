use clap::Parser;
use nexus_badges::{
    await_user_for_end,
    commands::{init_actions, init_remote, process, update_args, version, Modify},
    models::cli::{Cli, Commands},
    remote_unsupported,
    services::git::set_workflow_state,
    startup,
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
                remote_unsupported!(cli.remote, command);
                set_workflow_state(state)
                    .await
                    .unwrap_or_else(|err| eprintln!("{err}"));
                return;
            }
            _ => (),
        }
    }

    let input_mods = match startup(cli.remote) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("{err}");
            if !cli.remote {
                await_user_for_end();
            }
            return;
        }
    };

    if let Some(command) = cli.command {
        remote_unsupported!(cli.remote, command);
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
            Commands::Automation { state: _ } => unreachable!("by automation guard"),
            Commands::Version => unreachable!("by version guard"),
        }
    } else {
        process(input_mods, cli.remote)
            .await
            .unwrap_or_else(|err| eprintln!("{err}"));
        if !cli.remote {
            await_user_for_end();
        }
    }
}
