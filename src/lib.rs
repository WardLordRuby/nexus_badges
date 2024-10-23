pub mod cli;
pub mod error;
pub mod json_data;

// MARK: TODO
// Separate into modules

use crate::{
    cli::{Mod, SetArgs},
    error::Error,
    json_data::*,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use crypto_box::{aead::OsRng, PublicKey};
use percent_encoding::{percent_encode, AsciiSet, CONTROLS};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::BTreeMap,
    fs::File,
    io::{self, BufRead, BufReader, ErrorKind, Write},
    sync::{Arc, OnceLock},
};
use tokio::task::JoinSet;

const NEXUS_BASE_URL: &str = "https://api.nexusmods.com";
const GIT_BASE_URL: &str = "https://api.github.com";
const GIT_API_VER: &str = "2022-11-28";

const GIST_NAME: &str = "nexus_badges.json";
const GIST_DESC: &str = "Private gist to be used as a json endpoint for badge download counters";

const NEXUS_INFO_OK: u16 = 200;
const GIT_PUBLIC_KEY_OK: u16 = 200;
const GIT_SECRET_CREATED: u16 = 201;
const GIT_SECRET_UPDATED: u16 = 204;
const GIST_GET_OK: u16 = 200;
const GIST_UPDATE_OK: u16 = 200;
const GIST_CREATED: u16 = 201;

const RAW: &str = "/raw/";

const IO_DIR: &str = "io";
pub const INPUT_PATH: &str = "io/input.json";
pub const OUPUT_PATH: &str = "io/output.json";
const BADGES_PATH: &str = "io/badges.md";
const BADGES_PATH_LOCAL: &str = ".\\io\\badges.md";

const VERSION_URL: &str =
    "https://gist.githubusercontent.com/WardLordRuby/b7ae290f2a7f1a20e9795170965c4a46/raw/";

const ENV_NAME_NEXUS: &str = "NEXUS_KEY";
const ENV_NAME_GIT: &str = "GIT_TOKEN";

static VARS: OnceLock<StartupVars> = OnceLock::new();

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

impl Mod {
    fn get_info_endpoint(&self) -> String {
        format!(
            "{NEXUS_BASE_URL}/v1/games/{}/mods/{}.json",
            self.domain, self.mod_id
        )
    }
    fn url(&self) -> String {
        format!(
            "https://www.nexusmods.com/{}/mods/{}",
            self.domain, self.mod_id
        )
    }
}

fn gist_id_endpoint() -> String {
    format!(
        "{GIT_BASE_URL}/gists/{}",
        VARS.get().expect("set on startup").gist_id
    )
}

fn gist_endpoint() -> String {
    format!("{GIT_BASE_URL}/gists",)
}

fn repository_public_key_endpoint() -> String {
    let vars = VARS.get().expect("set on startup");
    format!(
        "{GIT_BASE_URL}/repos/{}/{}/actions/secrets/public-key",
        vars.owner, vars.repo
    )
}

fn repository_secret_endpoint(secret_name: &str) -> String {
    let vars = VARS.get().expect("set on startup");
    format!(
        "{GIT_BASE_URL}/repos/{}/{}/actions/secrets/{secret_name}",
        vars.owner, vars.repo
    )
}

fn git_token_h_key() -> String {
    format!("Bearer {}", VARS.get().expect("set on startup").git_token)
}

impl ModDetails {
    fn add_url(mut self, from: &Mod) -> Self {
        self.url = from.url();
        self
    }
}

impl GistResponse {
    fn file_details(&self) -> Result<&FileDetails, Error> {
        self.files.get(GIST_NAME).ok_or_else(|| {
            Error::BadResponse(format!(
                "Gist response did not contains details about any file with the name: {GIST_NAME}"
            ))
        })
    }

    fn universal_url(&self) -> Result<&str, Error> {
        self.file_details().map(|entry| {
            let i = entry.raw_url.find(RAW).expect("always contains `RAW`");
            &entry.raw_url[..i + RAW.len()]
        })
    }

    fn content(&self) -> Result<&str, Error> {
        self.file_details().map(|entry| entry.content.as_str())
    }
}

impl Input {
    pub fn add_mod(mut self, details: Mod) -> Result<(), Error> {
        if self.mods.contains(&details) {
            Err(Error::Io(io::Error::new(
                ErrorKind::InvalidInput,
                format!("Mod already exists in: {INPUT_PATH}"),
            )))
        } else {
            self.mods.push(details);
            write(self, INPUT_PATH)?;
            println!("Mod Registered!");
            Ok(())
        }
    }

    pub fn remove_mod(mut self, details: Mod) -> Result<(), Error> {
        if let Some(i) = self
            .mods
            .iter()
            .position(|mod_details| *mod_details == details)
        {
            self.mods.swap_remove(i);
            write(self, INPUT_PATH)?;
            println!("Mod removed!");
            Ok(())
        } else {
            Err(Error::Io(io::Error::new(
                ErrorKind::InvalidInput,
                format!("Mod does not exist in: {INPUT_PATH}"),
            )))
        }
    }

    // MARK: TODO
    // If actions are setup we should update the workflow after valid changes
    pub async fn update_args(mut self, mut new: SetArgs) -> Result<(), Error> {
        let try_update_secrets = verify_repo().is_ok();
        let (mut new_git_token, mut new_nexus_key) = (None, None);

        if let Some(token) = new.git {
            if try_update_secrets {
                new_git_token = Some(token.clone());
            }
            self.git_token = token;
        }
        if let Some(key) = new.nexus {
            if try_update_secrets {
                new_nexus_key = Some(key.clone());
            }
            self.nexus_key = key;
        }
        if let Some(ref mut id) = new.gist {
            std::mem::swap(&mut self.gist_id, id);
        }
        if let Some(repo) = new.repo {
            self.repo = repo;
        }
        if let Some(owner) = new.owner {
            self.owner = owner;
        }

        write(self, INPUT_PATH)?;

        if let Some(prev_id) = new.gist {
            if !prev_id.is_empty() {
                // MARK: XXX
                // Do we require confirmation for these kind of overwrites?
                println!("WARN: Previously stored gist_id: {prev_id}, was replaced");
            }
        }

        println!("Key(s) updated locally");

        if try_update_secrets && (new_git_token.is_some() || new_nexus_key.is_some()) {
            let public_key = get_public_key().await.map(Some).unwrap_or_else(|err| {
                eprintln!("{err}");
                None
            });

            let mut tasks = JoinSet::new();

            if let Some(key) = public_key {
                let key_arc = Arc::new(key);
                if let Some(secret) = new_git_token {
                    let key_clone = Arc::clone(&key_arc);
                    tasks.spawn(async move {
                        set_repository_secret(ENV_NAME_GIT, &secret, &key_clone).await
                    });
                }
                if let Some(secret) = new_nexus_key {
                    tasks.spawn(async move {
                        set_repository_secret(ENV_NAME_NEXUS, &secret, &key_arc).await
                    });
                }
            }

            while let Some(res) = tasks.join_next().await {
                if let Err(err) = res {
                    eprintln!("{err}")
                }
            }
        }

        Ok(())
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

async fn update_download_counts(mods: Vec<Mod>) -> Result<BTreeMap<u64, ModDetails>, Error> {
    verify_nexus()?;
    verify_added(&mods)?;

    let client = reqwest::Client::new();
    let mut tasks = JoinSet::new();

    for descriptor in mods.into_iter() {
        tasks.spawn(try_get_info(descriptor, client.clone()));
    }

    let mut output = BTreeMap::new();

    while let Some(res) = tasks.join_next().await {
        match res {
            Ok(Ok(data)) => {
                if let Some(dup) = output.insert(data.uid, data) {
                    tasks.abort_all();
                    while tasks.join_next().await.is_some() {}
                    return Err(Error::Io(io::Error::new(
                        ErrorKind::InvalidInput,
                        format!("duplicate tracked mod: {}, in: {INPUT_PATH}", dup.name),
                    )));
                }
            }
            Ok(Err(err)) => {
                tasks.abort_all();
                while tasks.join_next().await.is_some() {}
                return Err(err);
            }
            Err(_) => unreachable!("task can't panic"),
        }
    }

    write(output.clone(), OUPUT_PATH)?;

    println!(
        "Retrieved and saved locally download counts for {} mod(s)",
        output.len()
    );

    Ok(output)
}

pub async fn process(input: Input) -> Result<(), Error> {
    let (output_res, verify_res) = tokio::join!(update_download_counts(input.mods), verify_gist());

    let (gist_endpoint, prev_remote) = verify_res?;
    let output = output_res?;

    let new_content = serde_json::to_string_pretty(&output)?;

    if prev_remote.content()? != new_content {
        update_remote(&gist_endpoint, new_content).await?;
    } else {
        println!(
            "Download counts for tracked mod(s) have not changed, remote gist was not modified"
        );
    }

    let universal_url = prev_remote.universal_url()?;

    write_badges(output, universal_url)
}

pub async fn init_remote(input: Input) -> Result<(), Error> {
    verify_git()?;
    let mut updated_input = input.clone();
    let output = update_download_counts(input.mods).await?;

    let content = serde_json::to_string_pretty(&output)?;

    let server_response = reqwest::Client::new()
        .post(gist_endpoint())
        .headers(git_header())
        .json(&serde_json::json!({
            "description": GIST_DESC,
            "public": false,
            "files": {
                GIST_NAME: {
                    "content": content
                }
            }
        }))
        .send()
        .await?;

    if server_response.status() != GIST_CREATED {
        return Err(Error::BadResponse(server_response.text().await?));
    }

    println!("New private gist created with name: {GIST_NAME}");

    let mut meta = server_response.json::<GistResponse>().await?;

    if !input.gist_id.is_empty() && input.gist_id != meta.id {
        println!("Replacing gist_id: {}", input.gist_id);
    }

    println!("New gist_id: {}", meta.id);

    updated_input.gist_id = std::mem::take(&mut meta.id);
    write(updated_input, INPUT_PATH)?;

    write_badges(output, meta.universal_url()?)?;

    Ok(())
}

pub async fn init_actions(_input: Input) -> Result<(), Error> {
    verify_repo()?;

    let public_key = get_public_key().await?;
    let vars = VARS.get().expect("set on startup");
    let (res1, res2) = tokio::join!(
        set_repository_secret(ENV_NAME_GIT, &vars.git_token, &public_key),
        set_repository_secret(ENV_NAME_NEXUS, &vars.nexus_key, &public_key)
    );

    res1?;
    res2?;
    // MARK: TODO's
    // 1. Commit (non-sensitive) input.json
    // 2. Upload and schedule Action Workflow
    // 3. Build on multiple targets
    Ok(())
}

async fn update_remote(gist_endpoint: &str, content: String) -> Result<GistResponse, Error> {
    let server_response = reqwest::Client::new()
        .patch(gist_endpoint)
        .headers(git_header())
        .json(&serde_json::json!({
            "files": {
                GIST_NAME: {
                    "content": content
                }
            }
        }))
        .send()
        .await?;

    if server_response.status() != GIST_UPDATE_OK {
        return Err(Error::BadResponse(server_response.text().await?));
    }

    println!("Remote gist successfully updated");

    server_response
        .json::<GistResponse>()
        .await
        .map_err(Error::from)
}

async fn get_remote(gist_endpoint: &str) -> Result<GistResponse, Error> {
    let server_response = reqwest::Client::new()
        .get(gist_endpoint)
        .headers(git_header())
        .send()
        .await?;

    if server_response.status() != GIST_GET_OK {
        return Err(Error::BadResponse(server_response.text().await?));
    }

    server_response
        .json::<GistResponse>()
        .await
        .map_err(Error::from)
}

fn git_header() -> reqwest::header::HeaderMap {
    [
        ("User-Agent", Cow::Borrowed(env!("CARGO_PKG_NAME"))),
        ("Accept", Cow::Borrowed("application/vnd.github+json")),
        ("Authorization", Cow::Owned(git_token_h_key())),
        ("X-GitHub-Api-Version", Cow::Borrowed(GIT_API_VER)),
    ]
    .into_iter()
    .fold(HeaderMap::new(), |mut map, (key, cow)| {
        let val = match cow {
            Cow::Borrowed(b) => HeaderValue::from_static(b),
            Cow::Owned(o) => HeaderValue::from_str(&o).expect("git token produces valid key"),
        };
        assert!(map.insert(key, val).is_none());
        map
    })
}

async fn try_get_info(details: Mod, client: reqwest::Client) -> Result<ModDetails, Error> {
    let server_response = client
        .get(details.get_info_endpoint())
        .header("accept", "application/json")
        .header("apikey", &VARS.get().expect("set on startup").nexus_key)
        .send()
        .await?;

    if server_response.status() != NEXUS_INFO_OK {
        return Err(Error::BadResponse(server_response.text().await?));
    }

    server_response
        .json::<ModDetails>()
        .await
        .map(|output| output.add_url(&details))
        .map_err(Error::from)
}

async fn get_public_key() -> Result<RepositoryPublicKey, Error> {
    let server_response = reqwest::Client::new()
        .get(repository_public_key_endpoint())
        .headers(git_header())
        .send()
        .await?;

    if server_response.status() != GIT_PUBLIC_KEY_OK {
        return Err(Error::BadResponse(server_response.text().await?));
    }

    server_response
        .json::<RepositoryPublicKey>()
        .await
        .map_err(Error::from)
}

async fn set_repository_secret(
    secret_name: &str,
    secret: &str,
    public_key: &RepositoryPublicKey,
) -> Result<(), Error> {
    let encrypted_secret = encrypt_secret(secret, &public_key.key)?;

    let server_response = reqwest::Client::new()
        .put(repository_secret_endpoint(secret_name))
        .headers(git_header())
        .json(&serde_json::json!({
            "encrypted_value": encrypted_secret,
            "key_id": public_key.key_id,
        }))
        .send()
        .await?;

    if server_response.status() == GIT_SECRET_CREATED
        || server_response.status() == GIT_SECRET_UPDATED
    {
        println!(
            "Repository secret: {secret_name}, {}",
            match server_response.status() {
                s if s == GIT_SECRET_CREATED => "created",
                s if s == GIT_SECRET_UPDATED => "updated",
                _ => unreachable!("by outer if"),
            }
        );
        return Ok(());
    }

    Err(Error::BadResponse(server_response.text().await?))
}

fn encrypt_secret(secret: &str, public_key: &str) -> Result<String, Error> {
    let public_key = PublicKey::from_slice(&BASE64.decode(public_key)?).map_err(|err| {
        io::Error::new(ErrorKind::InvalidData, format!("Invalid public key: {err}"))
    })?;

    let encrypted_bytes = public_key.seal(&mut OsRng, secret.as_bytes())?;

    Ok(BASE64.encode(&encrypted_bytes))
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
