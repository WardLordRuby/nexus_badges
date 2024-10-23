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
        cli::Mod,
        error::Error,
        json_data::{GistResponse, Input, ModDetails, Version},
    },
    services::git::{get_remote, gist_id_endpoint},
};
use percent_encoding::{percent_encode, AsciiSet, CONTROLS};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
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

const VERSION_URL: &str =
    "https://gist.githubusercontent.com/WardLordRuby/b7ae290f2a7f1a20e9795170965c4a46/raw/";

static VARS: OnceLock<StartupVars> = OnceLock::new();

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

async fn check_program_version() -> reqwest::Result<()> {
    let version = reqwest::get(VERSION_URL).await?.json::<Version>().await?;
    if version.latest != env!("CARGO_PKG_VERSION") {
        println!("{}", version.message);
    }
    Ok(())
}

#[derive(Debug)]
struct StartupVars {
    nexus_key: String,
    git_token: String,
    gist_id: String,
    owner: String,
    repo: String,
}

impl From<&Input> for StartupVars {
    fn from(value: &Input) -> Self {
        let nexus_key = match std::env::var(ENV_NAME_NEXUS) {
            Ok(key) => key,
            Err(_) => value.nexus_key.clone(),
        };

        let git_token = match std::env::var(ENV_NAME_GIT) {
            Ok(key) => key,
            Err(_) => value.git_token.clone(),
        };

        StartupVars {
            nexus_key,
            git_token,
            gist_id: value.gist_id.clone(),
            owner: value.owner.clone(),
            repo: value.repo.clone(),
        }
    }
}

pub fn startup() -> Result<Input, Error> {
    tokio::task::spawn(check_program_version());

    if !std::fs::exists(IO_DIR)? {
        std::fs::create_dir(IO_DIR)?;
    }

    let input = match read::<Input>(INPUT_PATH) {
        Ok(data) => data,
        Err(err) => match err {
            Error::Io(err) if err.kind() == ErrorKind::NotFound => {
                eprintln!("Could not find: {INPUT_PATH}. Continuing with default data structure");
                Input::default()
            }
            _ => return Err(err),
        },
    };

    VARS.set(StartupVars::from(&input)).expect("only set");
    Ok(input)
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

pub fn await_user_for_end() {
    println!("Press enter to exit...");
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin);
    let _ = reader.read_line(&mut String::new());
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
