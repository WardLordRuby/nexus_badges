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

    /// Input Private keys
    #[command(alias = "set")]
    SetKey(SetKeyArgs),

    /// Initalize private gist to be used as a json endpoint for badge download counters
    Init,
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
pub struct SetKeyArgs {
    /// Github fine-grained private token (with r/w permission for gists)
    #[arg(long)]
    pub git: Option<String>,

    /// Nexus private api key
    #[arg(long)]
    pub nexus: Option<String>,
}
