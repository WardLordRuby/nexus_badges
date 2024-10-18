use crate::cli::{Cli, Commands};
use clap::Parser;
use nexus_badges::*;

#[tokio::main]
async fn main() {
    let mut input = match startup() {
        Ok(data) => data,
        Err(err) => {
            eprintln!("{err}");
            return;
        }
    };

    let cli = Cli::parse();

    if let Some(command) = cli.command {
        match command {
            Commands::SetKey(keys) => {
                input.update(keys);
                match write(input, INPUT_PATH) {
                    Ok(()) => println!("Key(s) updated"),
                    Err(err) => eprintln!("{err}"),
                }
            }
            Commands::Init => init_remote(input)
                .await
                .unwrap_or_else(|err| eprintln!("{err:?}")),
            Commands::Add(details) => {
                if input.mods.contains(&details) {
                    eprintln!("Mod already exists in: {INPUT_PATH}");
                } else {
                    input.mods.push(details);
                    match write(input, INPUT_PATH) {
                        Ok(()) => println!("Mod Registered!"),
                        Err(err) => eprintln!("{err}"),
                    }
                }
            }
            Commands::Remove(details) => {
                if let Some(i) = input
                    .mods
                    .iter()
                    .position(|mod_details| *mod_details == details)
                {
                    input.mods.swap_remove(i);
                    match write(input, INPUT_PATH) {
                        Ok(()) => println!("Mod removed!"),
                        Err(err) => eprintln!("{err}"),
                    }
                } else {
                    eprintln!("Mod does not exist in: {INPUT_PATH}");
                }
            }
        }
    } else {
        process(input)
            .await
            .unwrap_or_else(|err| eprintln!("{err}"))
    }
}
