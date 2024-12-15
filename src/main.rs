use clap::Parser;
use nexus_badges::{
    await_user_for_end,
    commands::{
        init_actions, init_remote, process, update_args_local, update_args_remote,
        update_cache_key, version, Modify,
    },
    exit_on_remote,
    models::{
        cli::{Cli, Commands},
        error::Error,
    },
    print_err, return_after,
    services::git::set_workflow_state,
    startup, unsupported,
};

#[tokio::main]
async fn main() {
    let mut cli = Cli::parse();

    if let Some(ref mut command) = cli.command {
        match command {
            Commands::Version => {
                return_after!(version(cli.remote).await, cli.remote);
            }
            Commands::UpdateCacheKey { old, new } => {
                unsupported!(command, on_local, cli.remote);
                return_after!(update_cache_key(old.as_deref(), new).await, cli.remote);
            }
            Commands::SetArg(args) => {
                unsupported!(command, on_remote, cli.remote);
                if let Err(err) = update_args_local(args).await {
                    if !matches!(err, Error::NotSetup(_)) {
                        eprintln!("{err}")
                    }
                    return;
                }
                if !args.modified.any() {
                    return;
                }
            }
            _ => (),
        }
    }

    let input_mods = match startup(cli.remote) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("{err}");
            exit_on_remote(cli.remote, 1);
            await_user_for_end(cli.remote);
            return;
        }
    };

    if let Some(command) = cli.command {
        unsupported!(command, on_remote, cli.remote);
        match command {
            Commands::SetArg(args) => print_err!(update_args_remote(args).await),
            Commands::Add(details) => print_err!(input_mods.add_mod(details).await),
            Commands::Remove(details) => print_err!(input_mods.remove_mod(details).await),
            Commands::Init => print_err!(init_remote(input_mods).await),
            Commands::InitActions => print_err!(init_actions(input_mods).await),
            Commands::Automation { state } => print_err!(set_workflow_state(state).await),
            Commands::UpdateCacheKey { old: _, new: _ } => unreachable!("by repo-variable guard"),
            Commands::Version => unreachable!("by version guard"),
        }
        return;
    }

    process(input_mods, cli.remote).await.unwrap_or_else(|err| {
        eprintln!("{err}");
        exit_on_remote(cli.remote, 1);
    });
    await_user_for_end(cli.remote);
}
