use crate::models::badge_options::{BadgeFormat, BadgeStyle, Color, DownloadCount};
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
    /// Add/Register a Nexus mod to track the download count of
    #[command(alias = "Add")]
    Add(Mod),

    /// Remove and stop tracking the download count of a registered mod
    #[command(alias = "Remove")]
    Remove(Mod),

    /// Configure necessary credentials for NexusMod and Git API calls
    /// {n}  and set badge style preferences
    #[command(aliases = ["Set", "set"])]
    SetArg(SetArgs),

    /// Initalize private gist to be used as a json endpoint for badge download counters
    #[command(alias = "Init")]
    Init,

    /// Initalize GitHub actions to update the remote gist once daily
    #[command(aliases = ["InitActions", "init_actions", "Init-Actions", "initActions"])]
    InitActions,

    /// Enable/Disable the GitHub action automation workflow
    #[command(alias = "Automation")]
    Automation {
        #[arg(value_enum)]
        state: Workflow,
    },

    /// Display current version and check for updates
    #[command(alias = "Version")]
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
    #[arg(short, long, alias = "game")]
    pub domain: String,

    /// The ID of the mod
    #[arg(short, long, alias = "id")]
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

    /// Identifier of the target remote Gist
    /// {n}  [This value is automatically set by the `init` command]
    #[arg(long, alias = "gist-id")]
    pub gist: Option<String>,

    /// Your GitHub user name [Required for GitHub actions setup]
    #[arg(long)]
    pub owner: Option<String>,

    /// Name of repository containing 'automation.yml' without the .git extension
    /// {n}  [Required for GitHub actions setup]
    #[arg(long)]
    pub repo: Option<String>,

    /// Specify a style to be added to badges [Default: flat]{n}  
    #[arg(long)]
    pub style: Option<BadgeStyle>,

    /// Specify download count to use [Default: total]
    #[arg(long)]
    pub count: Option<DownloadCount>,

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

    /// Specify the output format of the generated badges [Default: Markdown]{n}  
    #[arg(long)]
    pub format: Option<BadgeFormat>,

    #[clap(skip)]
    pub modified: ModFlags,
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
    #[value(alias = "Enable")]
    Enable,
    #[value(alias = "Disable")]
    Disable,
}
