pub mod commands;
pub mod models {
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
        cli::{Commands, Mod},
        error::Error,
        json_data::{GistResponse, Input, ModDetails, Version},
    },
    services::git::{get_remote, gist_id_endpoint},
};
use percent_encoding::{percent_encode, AsciiSet, CONTROLS};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt::Display,
    fs::File,
    io::{BufRead, BufReader, ErrorKind, Write},
    sync::OnceLock,
};

const IO_DIR: &str = "io";
pub const INPUT_PATH: &str = "io/input.json";
pub const OUPUT_PATH: &str = "io/output.json";
const BADGES_PATH: &str = "io/badges.md";
const BADGES_PATH_LOCAL: &str = ".\\io\\badges.md";

const ENV_NAME_NEXUS: &str = "NEXUS_KEY";
const ENV_NAME_GIT: &str = "GIT_TOKEN";
const ENV_NAME_GIST_ID: &str = "GIST_ID";
const ENV_NAME_MODS: &str = "TRACKED_MODS";

pub const OK_RESPONSE: u16 = 200;
pub const CREATED_RESPONSE: u16 = 201;
pub const UPDATED_RESPONSE: u16 = 204;

const VERSION_URL: &str =
    "https://gist.githubusercontent.com/WardLordRuby/b7ae290f2a7f1a20e9795170965c4a46/raw/";

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
        return Err(Error::Missing(
            "Use command 'init' to initialize a new remote gist",
        ));
    }
    let endpoint = gist_id_endpoint();
    let meta = get_remote(&endpoint).await?;
    Ok((endpoint, meta))
}

fn verify_repo() -> Result<(), Error> {
    let vars = VARS.get().expect("set on startup");
    if vars.repo.is_empty() {
        return Err(Error::Missing(
            "Use command 'set --repo' to input your forked 'nexus_badges'",
        ));
    }
    if vars.owner.is_empty() {
        return Err(Error::Missing(
            "Use command 'set --owner' to input your GitHub username",
        ));
    }
    Ok(())
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
        match read(INPUT_PATH) {
            Ok(data) => Ok(data),
            Err(err) => match err {
                Error::Io(err) if err.kind() == ErrorKind::NotFound => {
                    eprintln!(
                        "Could not find: {INPUT_PATH}. Continuing with default data structure"
                    );
                    Ok(Input::default())
                }
                _ => Err(err),
            },
        }
    }

    fn from_env() -> Result<Self, Error> {
        Ok(Input {
            git_token: std::env::var(ENV_NAME_GIT)?,
            nexus_key: std::env::var(ENV_NAME_NEXUS)?,
            gist_id: std::env::var(ENV_NAME_GIST_ID)?,
            owner: String::new(),
            repo: String::new(),
            mods: serde_json::from_str(&std::env::var(ENV_NAME_MODS)?)?,
        })
    }
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

    if !std::fs::exists(IO_DIR)? {
        std::fs::create_dir(IO_DIR)?;
    }

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

// MARK: TODO
// add badge formatter for url, rSt, AsciiDoc, HTML

fn write_badges(output: BTreeMap<u64, ModDetails>, universal_url: &str) -> Result<(), Error> {
    let mut file = File::create(BADGES_PATH)?;
    let encoded_url = percent_encode(universal_url.as_bytes(), CUSTOM_ENCODE_SET);

    for (uid, entry) in output.into_iter() {
        writeln!(file, "<!-- {} -->", entry.name)?;
        writeln!(file,
            "[![Nexus Downloads](https://img.shields.io/badge/dynamic/json?url={encoded_url}&query=%24.{uid}.mod_downloads&label=Nexus%20Downloads&labelColor=%2323282e)]({})",
            entry.url
        )?;
        writeln!(file)?;
    }

    println!("Badges saved to: {BADGES_PATH_LOCAL}");
    Ok(())
}

pub fn await_user_for_end(on_remote: bool) {
    if !on_remote {
        println!("Press enter to exit...");
        let stdin = std::io::stdin();
        let mut reader = BufReader::new(stdin);
        let _ = reader.read_line(&mut String::new());
    }
}

const CUSTOM_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`')
    .add(b'#')
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
