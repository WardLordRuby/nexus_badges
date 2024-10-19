pub mod cli;
pub mod error;
pub mod json_data;

const NEXUS_BASE_URL: &str = "https://api.nexusmods.com";
const GIST_API: &str = "https://api.github.com/gists";
const GIST_NAME: &str = "nexus_badges.json";
const GIT_API_VER: &str = "2022-11-28";

const NEXUS_INFO_OK: u16 = 200;
const GIST_UPDATE_OK: u16 = 200;
const GIST_CREATED: u16 = 201;

const RAW: &str = "/raw/";

const IO_DIR: &str = "io";
pub const INPUT_PATH: &str = "io/input.json";
pub const OUPUT_PATH: &str = "io/output.json";
const BADGES_PATH: &str = "io/badges.md";
const BADGES_PATH_LOCAL: &str = ".\\io\\badges.md";

static NEXUS_KEY: OnceLock<String> = OnceLock::new();
static GIT_TOKEN: OnceLock<String> = OnceLock::new();
static GIST_ID: OnceLock<String> = OnceLock::new();

use crate::{
    cli::{Mod, SetKeyArgs},
    error::Error,
    json_data::*,
};
use percent_encoding::{percent_encode, AsciiSet, CONTROLS};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::HashMap,
    fs::{read_to_string, File},
    io::{self, BufRead, BufReader, ErrorKind, Write},
    sync::OnceLock,
};

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

fn update_gist_endpoint() -> String {
    format!("{GIST_API}/{}", GIST_ID.get().expect("set on startup"))
}

fn git_token_h_key() -> String {
    format!("Bearer {}", GIT_TOKEN.get().expect("set on startup"))
}

impl ModDetails {
    fn add_url(mut self, from: &Mod) -> Self {
        self.url = from.url();
        self
    }
}

impl GistResponse {
    fn universal_url(&self) -> Result<&str, Error> {
        self.files
            .get(GIST_NAME)
            .ok_or_else(|| {
                Error::BadResponse(format!(
                    "Gist response did not contains details about any file with the name: {GIST_NAME}"
                ))
            })
            .map(|entry| {
                let i = entry.raw_url.find(RAW).expect("always contains `RAW`");
                &entry.raw_url[..i + RAW.len()]
            })
    }
}

impl Input {
    fn verify_nexus(&self) -> Result<(), Error> {
        if self.nexus_key.is_empty() {
            return Err(Error::Missing(
                "Nexus api key missing. Use command 'set' to store private key",
            ));
        }
        if self.git_token.is_empty() {
            println!(
                "Git fine-grained token missing, Use command 'set' to store private token\n\
                ouput will be saved locally"
            )
        }
        Ok(())
    }

    fn verify_added(&self) -> Result<(), Error> {
        if self.mods.is_empty() {
            return Err(Error::Missing(
                "No mods registered, use the command 'add' to register a mod",
            ));
        }
        Ok(())
    }

    fn verify_git(&self) -> Result<(), Error> {
        if self.git_token.is_empty() {
            return Err(Error::Missing(
                "Git fine-grained token missing, Use command 'set' to store private token\n\
                ouput will be saved locally",
            ));
        }
        Ok(())
    }

    fn verify_gist(&self) -> Result<(), Error> {
        if self.gist_id.is_empty() {
            return Err(Error::Missing(
                "Use command 'init' to initialize a new remote gist",
            ));
        }
        Ok(())
    }

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

    pub fn update_keys(mut self, new: SetKeyArgs) -> Result<(), Error> {
        if let Some(token) = new.git {
            self.git_token = token;
        }
        if let Some(key) = new.nexus {
            self.nexus_key = key;
        }
        write(self, INPUT_PATH)?;
        println!("Key(s) updated");
        Ok(())
    }
}

async fn update_download_counts(input: Input) -> Result<HashMap<u64, ModDetails>, Error> {
    input.verify_nexus()?;
    input.verify_added()?;

    let client = reqwest::Client::new();
    let tasks = input
        .mods
        .into_iter()
        .map(|details| tokio::task::spawn(try_get_info(details, client.clone())))
        .collect::<Vec<_>>();

    let mut output = HashMap::new();
    for task in tasks {
        let data = task.await.unwrap()?;
        assert!(
            output.insert(data.uid, data).is_none(),
            "duplicate entry in: {INPUT_PATH}"
        );
    }

    write(output.clone(), OUPUT_PATH)?;
    Ok(output)
}

pub async fn process(input: Input) -> Result<(), Error> {
    let gist_set = input.verify_gist();

    let output = update_download_counts(input).await?;
    gist_set?;

    let gist_meta = update_remote().await?;
    let universal_url = gist_meta.universal_url()?;

    println!(
        "Remote gist successfully updated with download counts for {} mod(s)",
        output.len()
    );

    write_badges(output, universal_url)
}

pub async fn init_remote(mut input: Input) -> Result<(), Error> {
    input.verify_git()?;
    let output = update_download_counts(input.clone()).await?;

    let processed_output = read_to_string(OUPUT_PATH)?;
    let body = serde_json::to_string(&GistNew::from(Upload::from(processed_output)))?;

    let server_response = reqwest::Client::new()
        .post(GIST_API)
        .headers(gist_header())
        .body(body)
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

    input.gist_id = std::mem::take(&mut meta.id);
    write(input, INPUT_PATH)?;

    write_badges(output, meta.universal_url()?)?;

    Ok(())
}

async fn update_remote() -> Result<GistResponse, Error> {
    let processed_output = read_to_string(OUPUT_PATH)?;
    let body = serde_json::to_string(&GistUpdate::from(Upload::from(processed_output)))?;

    let server_response = reqwest::Client::new()
        .patch(update_gist_endpoint())
        .headers(gist_header())
        .body(body)
        .send()
        .await?;

    if server_response.status() != GIST_UPDATE_OK {
        return Err(Error::BadResponse(server_response.text().await?));
    }

    server_response
        .json::<GistResponse>()
        .await
        .map_err(Error::from)
}

fn gist_header() -> reqwest::header::HeaderMap {
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
        .header("apikey", NEXUS_KEY.get().unwrap())
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

pub fn startup() -> Result<Input, Error> {
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

    NEXUS_KEY.set(input.nexus_key.clone()).expect("only set");
    GIT_TOKEN.set(input.git_token.clone()).expect("only set");
    GIST_ID.set(input.gist_id.clone()).expect("only set");
    Ok(input)
}

pub fn read<T: for<'de> Deserialize<'de>>(path: &str) -> Result<T, Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let data = serde_json::from_reader(reader)?;
    Ok(data)
}

pub fn write<T: Serialize>(data: T, file: &str) -> Result<(), Error> {
    let file = File::create(file)?;
    serde_json::to_writer_pretty(file, &data)?;
    Ok(())
}

fn write_badges(output: HashMap<u64, ModDetails>, universal_url: &str) -> Result<(), Error> {
    let mut file = File::create(BADGES_PATH)?;
    let encoded_url = percent_encode(universal_url.as_bytes(), CUSTOM_ENCODE_SET).to_string();

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
