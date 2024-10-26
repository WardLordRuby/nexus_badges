use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
pub struct Cli {
    // Nested layer of optional subcommands
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(long, action = ArgAction::SetTrue, hide = true)]
    pub remote: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Add Mod to input data
    Add(Mod),

    /// Remove mod from input data
    Remove(Mod),

    /// Configure necessary credentials for NexusMod and Git API calls
    #[command(alias = "set")]
    SetArg(SetArgs),

    /// Initalize private gist to be used as a json endpoint for badge download counters
    Init,

    /// Initalize GitHub actions to run the binary at scheduled times
    InitActions,

    /// Set the state for download counter automation via GitHub actions
    Automation {
        #[arg(value_enum)]
        state: Workflow,
    },

    /// Display current version
    Version,

    /// Remove previous cache and update the cache repository variable [Not supported on local]
    #[command(hide = true)]
    UpdateCacheKey {
        /// Cache Key to be deleted
        #[arg(long)]
        old: Option<String>,

        /// Cache repository variable to be updated
        #[arg(long)]
        new: String,
    },
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
    /// Github fine-grained private token with read/write permissions for gists
    /// {n}  [Additional permissions are required for automation via GitHub actions
    /// {n}  refer to documentation at https://github.com/WardLordRuby/nexus_badges]
    #[arg(long, alias = "git-token")]
    pub git: Option<String>,

    /// Nexus private api key
    #[arg(long, alias = "nexus-key")]
    pub nexus: Option<String>,

    /// Identifier of the target Remote Gist
    #[arg(long, alias = "gist-id")]
    pub gist: Option<String>,

    /// Your GitHub user name [Required for GitHub actions setup]
    #[arg(long)]
    pub owner: Option<String>,

    /// Name of your forked repository of 'nexus_badges' without the .git extension
    /// {n}  [Required for GitHub actions setup]
    #[arg(long)]
    pub repo: Option<String>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum Workflow {
    Enable,
    Disable,
}
