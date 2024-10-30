use crate::models::badge_options::{BadgeStyle, Color, DownloadCount};
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

    /// Initalize GitHub actions to update the remote gist once daily
    InitActions,

    /// Enable/Disable the GitHub action automation workflow
    Automation {
        #[arg(value_enum)]
        state: Workflow,
    },

    /// Display current version and check for updates
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

#[derive(Args, Debug, Default)]
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

    /// Specify a style to be added to badges [Default: flat]{n}  
    #[arg(long)]
    pub style: Option<BadgeStyle>,

    /// Specify download count to use [Default: total]
    #[arg(long)]
    pub counter: Option<DownloadCount>,

    /// Specify label to use on badges [Default: 'Nexus Downloads']
    /// {n}  [Tip: use quotes to include spaces]
    #[arg(long)]
    pub label: Option<String>,

    /// Specify a hex color for label side of the badge
    /// {n}  [Tip: input colors as '#23282e' or 23282e]
    #[arg(long)]
    pub label_color: Option<Color>,

    /// Specify a hex color for counter side of the badge
    /// {n} [Tip: to remove a color set as default]
    #[arg(long)]
    pub color: Option<Color>,

    #[clap(skip)]
    pub modified: ModFlags,
}

impl SetArgs {
    #[inline]
    pub fn key_modified(&self) -> bool {
        self.git.is_some()
            || self.nexus.is_some()
            || self.gist.is_some()
            || self.owner.is_some()
            || self.repo.is_some()
    }

    #[inline]
    pub fn pref_modified(&self) -> bool {
        self.style.is_some()
            || self.counter.is_some()
            || self.label.is_some()
            || self.label_color.is_some()
            || self.color.is_some()
    }
}

#[derive(Debug, Default)]
pub struct ModFlags {
    pub git_token: bool,
    pub nexus_key: bool,
    pub gist_id: bool,
}

impl ModFlags {
    #[inline]
    pub fn any(&self) -> bool {
        self.git_token || self.nexus_key || self.gist_id
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum Workflow {
    Enable,
    Disable,
}
