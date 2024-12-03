pub mod commands;
pub mod models {
    pub mod badge_options;
    pub mod cli;
    pub mod error;
    pub mod json_data;
}
pub mod services {
    pub mod git;
    pub mod nexus;
}

use crate::{
    models::{
        badge_options::{BadgePreferences, EncodedFields},
        cli::{Commands, Mod},
        error::Error,
        json_data::{GistResponse, Input, ModDetails, Version},
    },
    services::git::{get_remote, gist_id_endpoint},
};
use constcat::concat;
use percent_encoding::{AsciiSet, CONTROLS};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::BTreeMap,
    fmt::Display,
    fs::File,
    io::{self, BufRead, BufReader, BufWriter, ErrorKind, Write},
    sync::{LazyLock, OnceLock},
};

const DEFAULT_IO_DIR_NAME: &str = "io";
const INPUT_FILE_NAME: &str = "input.json";
const OUTPUT_FILE_NAME: &str = "output.json";
const PREFERENCES_FILE_NAME: &str = "badge_preferences.json";
const BADGES_FILE_NAME: &str = "badges.md";

pub static PATHS: LazyLock<FilePaths> = LazyLock::new(init_paths);

const BADGE_URL: &str = "https://shields.io/badges/dynamic-json-badge";

const ENV_NAME_NEXUS: &str = "NEXUS_KEY";
const ENV_NAME_GIT: &str = "GIT_TOKEN";
const ENV_NAME_GIST_ID: &str = "GIST_ID";
const ENV_NAME_MODS: &str = "TRACKED_MODS";

pub const OK_RESPONSE: u16 = 200;
pub const CREATED_RESPONSE: u16 = 201;
pub const UPDATED_RESPONSE: u16 = 204;

const VERSION_URL: &str =
    "https://gist.githubusercontent.com/WardLordRuby/b7ae290f2a7f1a20e9795170965c4a46/raw";

pub const TOTAL_KEY: &str = "Totals";

static VARS: OnceLock<StartupVars> = OnceLock::new();

#[macro_export]
macro_rules! unsupported {
    ($command:ident, on_remote, $on_remote:expr) => {
        if $on_remote {
            eprintln!("'{}' is not supported on remote", $command);
            std::process::exit(95)
        }
    };

    ($command:ident, on_local, $on_remote:expr) => {
        if !$on_remote {
            eprintln!("'{}' is only supported on remote", $command);
            return;
        }
    };
}

#[macro_export]
macro_rules! return_after {
    ($result:expr, $on_remote:expr) => {
        $result.unwrap_or_else(|err| {
            eprintln!("{err}");
            if $on_remote {
                std::process::exit(1)
            }
        });
        return;
    };
}

#[macro_export]
macro_rules! print_err {
    ($result:expr) => {
        $result.unwrap_or_else(|err| eprintln!("{err}"))
    };
}

pub struct FilePaths {
    pub input: Cow<'static, str>,
    pub output: Cow<'static, str>,
    pub badges: Cow<'static, str>,
    pub preferences: Cow<'static, str>,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn init_paths() -> FilePaths {
    const UNIX_INSTALL: &str = "usr/local/bin";

    let mut exe_dir = match std::env::current_exe() {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!(
                "Could not locate executable, {err}\n\
                Using executable local paths for input + output"
            );
            return FilePaths::default();
        }
    };

    exe_dir.pop();

    if !exe_dir.ends_with(UNIX_INSTALL) {
        return FilePaths::default();
    }

    let home = std::env::var("HOME").expect("valid var on unix");

    #[cfg(target_os = "linux")]
    let base = format!(
        "{home}/.config/{}",
        env!("CARGO_PKG_NAME").replace('_', "-")
    );

    #[cfg(target_os = "macos")]
    let base = format!(
        "{home}/Library/{}",
        camel_case(env!("CARGO_PKG_NAME"), true)
    );

    FilePaths {
        input: Cow::Owned(format!("{base}/{INPUT_FILE_NAME}")),
        output: Cow::Owned(format!("{base}/{OUTPUT_FILE_NAME}")),
        badges: Cow::Owned(format!("{home}/Documents/{BADGES_FILE_NAME}")),
        preferences: Cow::Owned(format!("{base}/{PREFERENCES_FILE_NAME}")),
    }
}

#[cfg(target_os = "windows")]
const fn init_paths() -> FilePaths {
    FilePaths::default()
}

impl FilePaths {
    const fn default() -> Self {
        FilePaths {
            input: Cow::Borrowed(concat!(DEFAULT_IO_DIR_NAME, "/", INPUT_FILE_NAME)),
            output: Cow::Borrowed(concat!(DEFAULT_IO_DIR_NAME, "/", OUTPUT_FILE_NAME)),
            badges: Cow::Borrowed(concat!(DEFAULT_IO_DIR_NAME, "/", BADGES_FILE_NAME)),
            preferences: Cow::Borrowed(concat!(DEFAULT_IO_DIR_NAME, "/", PREFERENCES_FILE_NAME)),
        }
    }
}

#[cfg(target_os = "macos")]
fn camel_case(input: &str, capitalize_first: bool) -> String {
    const SEPARATORS: [char; 2] = ['-', '_'];

    let mut capitalize_next = false;
    let input = input.to_lowercase();
    input
        .trim()
        .trim_matches(SEPARATORS)
        .char_indices()
        .filter_map(|(i, ch)| {
            if i == 0 {
                return Some(if capitalize_first {
                    ch.to_ascii_uppercase()
                } else {
                    ch
                });
            }
            if ch.is_whitespace() || SEPARATORS.iter().any(|&s| s == ch) {
                capitalize_next = true;
                return None;
            }
            if capitalize_next {
                capitalize_next = false;
                return Some(ch.to_ascii_uppercase());
            }
            Some(ch)
        })
        .collect()
}

impl Display for Commands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Commands::Add(_) => "add",
                Commands::Remove(_) => "remove",
                Commands::SetArg(_) => "set-arg",
                Commands::Automation { state: _ } => "automation",
                Commands::Init => "init",
                Commands::InitActions => "init-actions",
                Commands::Version => "version",
                Commands::UpdateCacheKey { old: _, new: _ } => "repo-variable",
            }
        )
    }
}

impl ModDetails {
    fn total() -> Self {
        ModDetails {
            name: String::from("Sum of all tracked counts"),
            ..Default::default()
        }
    }

    fn add(&mut self, other: &Self) {
        self.mod_downloads += other.mod_downloads;
        self.mod_unique_downloads += other.mod_unique_downloads;
    }

    fn add_url(mut self, from: &Mod) -> Self {
        self.url = from.url();
        self
    }
}

fn verify_added(mods: &[Mod]) -> Result<(), Error> {
    if mods.is_empty() {
        return Err(Error::Missing(
            "No mods registered, use the command 'add' to register a mod",
        ));
    }
    Ok(())
}

fn verify_nexus() -> Result<(), Error> {
    let vars = VARS.get().expect("set on startup");
    if vars.nexus_key.is_empty() {
        return Err(Error::Missing(
            "Nexus api key missing. Use command 'set' to store private key",
        ));
    }
    if vars.git_token.is_empty() {
        println!(
            "Git fine-grained token missing, Use command 'set' to store private token\n\
            ouput will be saved locally"
        )
    }
    Ok(())
}

fn verify_git() -> Result<(), Error> {
    if VARS.get().expect("set on startup").git_token.is_empty() {
        return Err(Error::Missing(
            "Git fine-grained token missing, Use command 'set' to store private token",
        ));
    }
    Ok(())
}

async fn verify_gist() -> Result<(String, GistResponse), Error> {
    if VARS.get().expect("set on startup").gist_id.is_empty() {
        return Err(Error::NotSetup(
            "Use command 'init' to initialize a new remote gist",
        ));
    }
    let endpoint = gist_id_endpoint();
    let meta = get_remote(&endpoint).await?;
    Ok((endpoint, meta))
}

fn verify_repo_from(owner: &str, repo: &str) -> Result<(), Error> {
    if repo.is_empty() && owner.is_empty() {
        return Err(Error::NotSetup(
            "No repository set as target location of 'automation.yml' workflow.\n\
            To setup automation workflow use commands:\n\
            - 'nexus_badges.exe set-arg --owner <GITHUB_NAME> --repo <REPOSITORY_NAME>'\n
            - 'nexus_badges.exe init-actions'",
        ));
    }
    if repo.is_empty() {
        return Err(Error::Missing(
            "Use command 'set --repo' to input your forked 'nexus_badges'",
        ));
    }
    if owner.is_empty() {
        return Err(Error::Missing(
            "Use command 'set --owner' to input your GitHub username",
        ));
    }
    Ok(())
}

fn verify_repo() -> Result<(), Error> {
    let vars = VARS.get().expect("set on startup");
    verify_repo_from(&vars.owner, &vars.repo)
}

async fn check_program_version() -> reqwest::Result<Option<String>> {
    let version = reqwest::get(VERSION_URL).await?.json::<Version>().await?;
    if version.latest != env!("CARGO_PKG_VERSION") {
        return Ok(Some(version.message));
    }
    Ok(None)
}

#[derive(Debug, Default)]
pub struct StartupVars {
    nexus_key: String,
    git_token: String,
    gist_id: String,
    owner: String,
    repo: String,
}

impl StartupVars {
    /// NOTE: this method is not supported on local  
    /// `nexus_key` and `gist_id` fields are not populated from enviorment variables
    pub fn git_api_only() -> Result<Self, Error> {
        const ENV_NAME_REPO: &str = "REPO_FULL";

        let (owner, repo) = std::env::var(ENV_NAME_REPO)?
            .split_once('/')
            .map(|(owner, repo)| (owner.to_string(), repo.to_string()))
            .expect("'github.repository' is always formatted with a '/'");
        Ok(StartupVars {
            git_token: std::env::var(ENV_NAME_GIT)?,
            owner,
            repo,
            ..Default::default()
        })
    }
}

impl From<&mut Input> for StartupVars {
    fn from(value: &mut Input) -> Self {
        StartupVars {
            nexus_key: std::mem::take(&mut value.nexus_key),
            git_token: std::mem::take(&mut value.git_token),
            gist_id: std::mem::take(&mut value.gist_id),
            owner: std::mem::take(&mut value.owner),
            repo: std::mem::take(&mut value.repo),
        }
    }
}

impl Input {
    pub fn from(startup: &StartupVars, mods: Vec<Mod>) -> Self {
        Input {
            git_token: startup.git_token.clone(),
            nexus_key: startup.nexus_key.clone(),
            gist_id: startup.gist_id.clone(),
            owner: startup.owner.clone(),
            repo: startup.repo.clone(),
            mods,
        }
    }

    fn from_file() -> Result<Self, Error> {
        match read(&PATHS.input) {
            Ok(data) => Ok(data),
            Err(err) => match err {
                Error::Io(err) if err.kind() == ErrorKind::NotFound => {
                    eprintln!(
                        "Could not find: {INPUT_FILE_NAME}. Continuing with default data structure"
                    );
                    Ok(Input::default())
                }
                _ => Err(err),
            },
        }
    }

    /// `owner` and `repo` fields are not populated from enviorment variables
    fn from_env() -> Result<Self, Error> {
        Ok(Input {
            git_token: std::env::var(ENV_NAME_GIT)?,
            nexus_key: std::env::var(ENV_NAME_NEXUS)?,
            gist_id: std::env::var(ENV_NAME_GIST_ID)?,
            mods: serde_json::from_str(&std::env::var(ENV_NAME_MODS)?)?,
            ..Default::default()
        })
    }
}

fn prep_io_paths() -> io::Result<()> {
    let config_dir = PATHS.input.rsplit_once('/').expect("has forward slash").0;
    if !std::fs::exists(config_dir)? {
        std::fs::create_dir_all(config_dir)?;
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let output_dir = PATHS.badges.rsplit_once('/').expect("has forward slash").0;
        if !std::fs::exists(output_dir)? {
            std::fs::create_dir_all(output_dir)?;
        }
    }
    Ok(())
}

pub fn startup(on_remote: bool) -> Result<Vec<Mod>, Error> {
    if !on_remote {
        tokio::task::spawn(async {
            match check_program_version().await {
                Ok(Some(msg)) => println!("{msg}"),
                Ok(None) => (),
                Err(err) => eprintln!("{err}"),
            }
        });
    }

    prep_io_paths()?;

    let mut input = if on_remote {
        Input::from_env()
    } else {
        Input::from_file()
    }?;

    VARS.set(StartupVars::from(&mut input)).expect("only set");
    Ok(input.mods)
}

pub fn read<T: for<'de> Deserialize<'de>>(path: &str) -> Result<T, Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let data = serde_json::from_reader(reader)?;
    Ok(data)
}

pub fn write<T: Serialize>(data: T, path: &str) -> Result<(), Error> {
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, &data)?;
    Ok(())
}

fn write_badges(output: BTreeMap<String, ModDetails>, universal_url: &str) -> Result<(), Error> {
    let file = File::create(PATHS.badges.as_ref())?;
    let mut writer = BufWriter::new(file);

    let badge_prefs = read::<BadgePreferences>(&PATHS.preferences).unwrap_or_else(|err| {
        if !matches!(&err, Error::Io(err) if err.kind() == ErrorKind::NotFound) {
            eprintln!("{err}, using default styling")
        }
        BadgePreferences::default()
    });

    let encoded_fields = EncodedFields::new(universal_url, &badge_prefs, URL_ENCODE_SET);

    writeln!(writer, "# Shields.io Badges via Nexus Badges")?;
    writeln!(writer, "Base template: {BADGE_URL}")?;
    writeln!(writer, "Data source URL: {universal_url}")?;
    writeln!(writer, "{badge_prefs}")?;

    for (uid, entry) in output.into_iter() {
        let query = format!("$.{uid}.{}", badge_prefs.count.field_name());
        writeln!(writer, "## {}", entry.name)?;
        badge_prefs.format.write_badge(
            &mut writer,
            URL_ENCODE_SET,
            &encoded_fields,
            &query,
            &entry.url,
        )?;
        writeln!(writer)?;
        writeln!(writer, "Configuration:")?;
        writeln!(writer, "- Query: {query}")?;
        if !entry.url.is_empty() {
            writeln!(writer, "- Link: {}", entry.url)?;
        }
        writeln!(writer)?;
    }

    writer.flush()?;

    println!("Badges saved to: {}", PATHS.badges);
    Ok(())
}

pub fn await_user_for_end(on_remote: bool) {
    if !on_remote {
        println!("Press enter to exit...");
        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin);
        let _ = reader.read_line(&mut String::new());
    }
}

const URL_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'!')
    .add(b'#')
    .add(b'$')
    .add(b'%')
    .add(b'&')
    .add(b'\'')
    .add(b'(')
    .add(b')')
    .add(b'*')
    .add(b'+')
    .add(b',')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`')
    .add(b'?')
    .add(b'{')
    .add(b'}')
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'=')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'|');

pub async fn conditional_join<T1, T2>(
    task1: Option<impl std::future::Future<Output = T1>>,
    task2: Option<impl std::future::Future<Output = T2>>,
) -> (Option<T1>, Option<T2>) {
    match (task1, task2) {
        (Some(t1), Some(t2)) => {
            let (r1, r2) = tokio::join!(t1, t2);
            (Some(r1), Some(r2))
        }
        (Some(t1), None) => (Some(t1.await), None),
        (None, Some(t2)) => (None, Some(t2.await)),
        (None, None) => (None, None),
    }
}
