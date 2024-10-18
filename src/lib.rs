pub mod cli;
pub mod error;
pub mod json_data;

const NEXUS_BASE_URL: &str = "https://api.nexusmods.com";
const GIST_ENDPOINT: &str = "https://api.github.com/gists";
const GIST_NAME: &str = "nexus_badges.json";
const GIT_API_VER: &str = "2022-11-28";
const IO_DIR: &str = "io";
pub const INPUT_PATH: &str = "io/input.json";
pub const OUPUT_PATH: &str = "io/output.json";

static NEXUS_KEY: OnceLock<String> = OnceLock::new();
static GIT_TOKEN: OnceLock<String> = OnceLock::new();

use crate::{
    cli::{Mod, SetKeyArgs},
    error::Error,
    json_data::*,
};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, ErrorKind},
    sync::OnceLock,
};

fn mod_details_endpoint(details: &Mod) -> String {
    format!(
        "{NEXUS_BASE_URL}/v1/games/{}/mods/{}.json",
        details.domain, details.mod_id
    )
}

fn update_gist_endpoint(id: &str) -> String {
    format!("{GIST_ENDPOINT}/{id}")
}

fn git_token_h_key() -> String {
    format!("Bearer {}", GIT_TOKEN.get().expect("set on startup"))
}

impl Input {
    pub fn verify_nexus(&self) -> Result<(), Error> {
        if self.nexus_key.is_empty() {
            return Err(Error::Missing(
                "Nexus api key missing. Use 'set' to store private key",
            ));
        }
        if self.git_token.is_empty() {
            println!(
                "Git fine-grained token missing, Use 'set' to store private token\n\
                ouput will be saved locally"
            )
        }
        Ok(())
    }

    pub fn verify_git(&self) -> Result<(), Error> {
        if self.git_token.is_empty() {
            return Err(Error::Missing(
                "Git fine-grained token missing, Use 'set' to store private token\n\
                ouput will be saved locally",
            ));
        }
        if !self.gist_id.is_empty() {
            println!("Replacing gist_id: {}", self.gist_id);
        }
        Ok(())
    }

    pub fn verify_gist(&self) -> Result<(), Error> {
        if self.gist_id.is_empty() {
            return Err(Error::Missing(
                "Use command 'init' to initialize a new remote gist",
            ));
        }
        Ok(())
    }

    pub fn update(&mut self, input: SetKeyArgs) {
        if let Some(token) = input.git {
            self.git_token = token;
        }
        if let Some(key) = input.nexus {
            self.nexus_key = key;
        }
    }
}

pub async fn process(input: Input) -> Result<(), Error> {
    input.verify_nexus()?;
    let remote_gist = input.verify_gist();
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
    let count = output.len();

    write(output, OUPUT_PATH)?;

    remote_gist?;
    update_remote(input.gist_id).await?;

    println!("Remote gist successfully updated with download counts for {count} mod(s)");
    await_user_for_end();
    Ok(())
}

pub async fn init_remote(mut input: Input) -> Result<(), Error> {
    input.verify_git()?;
    process(input.clone()).await?;

    let processed_output = std::fs::read_to_string(OUPUT_PATH)?;
    let body = serde_json::to_string(&GistNew::from(Upload::from(processed_output)))?;

    let server_response = reqwest::Client::new()
        .post(GIST_ENDPOINT)
        .headers(gist_header())
        .body(body)
        .send()
        .await?;

    if server_response.status() != 201 {
        eprintln!("{}", server_response.text().await?);
        return Ok(());
    }

    let meta = server_response.json::<GistResponse>().await?;
    input.gist_id = meta.id;
    write(input, INPUT_PATH)?;

    Ok(())
}

async fn update_remote(gist_id: String) -> Result<(), Error> {
    let processed_output = std::fs::read_to_string(OUPUT_PATH)?;
    let body = serde_json::to_string(&GistUpdate::from(Upload::from(processed_output)))?;

    let server_response = reqwest::Client::new()
        .patch(update_gist_endpoint(&gist_id))
        .headers(gist_header())
        .body(body)
        .send()
        .await?;

    if server_response.status() != 200 {
        eprintln!("{}", server_response.text().await?);
        return Ok(());
    }

    Ok(())
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

async fn try_get_info(details: Mod, client: reqwest::Client) -> reqwest::Result<ModDetails> {
    let server_responce = client
        .get(mod_details_endpoint(&details))
        .header("accept", "application/json")
        .header("apikey", NEXUS_KEY.get().unwrap())
        .send()
        .await?;

    server_responce.json::<ModDetails>().await
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
    Ok(input)
}

pub fn read<T: for<'de> Deserialize<'de>>(path: &str) -> Result<T, Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let data = serde_json::from_reader(reader)?;
    Ok(data)
}

pub fn write<T: Serialize>(data: T, file: &str) -> Result<(), Error> {
    let file = std::fs::File::create(file)?;
    serde_json::to_writer_pretty(file, &data)?;
    Ok(())
}

fn await_user_for_end() {
    println!("Press enter to exit...");
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin);
    let _ = reader.read_line(&mut String::new());
}
