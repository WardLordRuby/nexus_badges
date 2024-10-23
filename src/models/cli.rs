use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(version)]
pub struct Cli {
    // Nested layer of optional subcommands
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Add Mod to input data
    Add(Mod),

    /// Remove mod from input data
    Remove(Mod),

    /// Input Private keys to be stored locally (or uploaded as Repository secrets)
    #[command(alias = "set")]
    SetArg(SetArgs),

    /// Initalize private gist to be used as a json endpoint for badge download counters
    Init,

    /// Initalize GitHub actions to run the binary at scheduled times
    InitActions,
}

#[derive(Args, Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
pub struct Mod {
    /// The name of the game the mod is made for
    #[arg(short, long)]
    pub domain: String,

    /// The ID of the mod
    #[arg(short, long)]
    pub mod_id: usize,
}

#[derive(Args, Debug)]
#[group(multiple = true, required = true)]
pub struct SetArgs {
    /// Github fine-grained private token (with r/w permission for gists)
    #[arg(long, alias = "git-token")]
    pub git: Option<String>,

    /// Nexus private api key
    #[arg(long, alias = "nexus-key")]
    pub nexus: Option<String>,

    /// Identifier of the target Remote Gist
    #[arg(long, alias = "gist-id")]
    pub gist: Option<String>,

    /// Your GitHub user name
    /// {n}  (Required for GitHub actions setup)
    #[arg(long)]
    pub owner: Option<String>,

    /// Name of your forked repository to 'nexus_badges' without the .git extension
    /// {n}  (Required for GitHub actions setup)
    #[arg(long)]
    pub repo: Option<String>,
}
