use crate::{
    CREATED, OK, VARS,
    models::{
        cli::Workflow,
        error::Error,
        json_data::{FileDetails, GistResponse, RepositoryPublicKey},
    },
    verify_repo,
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use crypto_box::{PublicKey, aead::OsRng};
use reqwest::header::{HeaderMap, HeaderValue};
use std::{
    borrow::Cow,
    fmt::Display,
    io::{self, ErrorKind},
};

const GIT_BASE_URL: &str = "https://api.github.com";
const GIT_API_VER: &str = "2022-11-28";

const GIST_NAME: &str = "nexus_badges.json";
const GIST_DESC: &str = "Private gist to be used as a json endpoint for badge download counters";

const WORKFLOW_NAME: &str = "automation.yml";
const RAW: &str = "/raw/";

// GitHub REST API uses 'No Content'(204) for the successful return of updated content and flag changes
const UPDATED: reqwest::StatusCode = reqwest::StatusCode::NO_CONTENT;

impl GistResponse {
    fn file_details(&self) -> Result<&FileDetails, Error> {
        self.files.get(GIST_NAME).ok_or_else(|| {
            Error::BadResponse(format!(
                "Gist response did not contains details about any file with the name: {GIST_NAME}"
            ))
        })
    }

    pub(crate) fn universal_url(&self) -> Result<&str, Error> {
        self.file_details().map(|entry| {
            let i = entry.raw_url.find(RAW).expect("always contains `RAW`");
            &entry.raw_url[..i + (RAW.len() - '/'.len_utf8())]
        })
    }

    pub(crate) fn content(&self) -> Result<&str, Error> {
        self.file_details().map(|entry| entry.content.as_str())
    }
}

pub(crate) fn gist_id_endpoint() -> String {
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

fn repository_variables_endpoint() -> String {
    let vars = VARS.get().expect("set on startup");
    format!(
        "{GIT_BASE_URL}/repos/{}/{}/actions/variables",
        vars.owner, vars.repo
    )
}

fn repository_variable_endpoint(var: &str) -> String {
    let vars = VARS.get().expect("set on startup");
    format!(
        "{GIT_BASE_URL}/repos/{}/{}/actions/variables/{var}",
        vars.owner, vars.repo
    )
}

fn repository_cache_endpoint(key: &str) -> String {
    let vars = VARS.get().expect("set on startup");
    format!(
        "{GIT_BASE_URL}/repos/{}/{}/actions/caches?key={key}",
        vars.owner, vars.repo
    )
}

fn workflow_endpoint_state(state: Workflow) -> String {
    let vars = VARS.get().expect("set on startup");
    format!(
        "{GIT_BASE_URL}/repos/{}/{}/actions/workflows/{WORKFLOW_NAME}/{state}",
        vars.owner, vars.repo
    )
}

impl Display for Workflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Workflow::Enable => "enable",
                Workflow::Disable => "disable",
            }
        )
    }
}

fn git_token_h_key() -> String {
    format!("Bearer {}", VARS.get().expect("set on startup").git_token)
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
            Cow::Owned(o) => {
                HeaderValue::from_str(&o).expect("`git_token_h_key` produces valid key")
            }
        };
        map.insert(key, val);
        map
    })
}

pub async fn set_workflow_state(state: Workflow) -> Result<(), Error> {
    verify_repo()?;

    let server_response = reqwest::Client::new()
        .put(workflow_endpoint_state(state))
        .headers(git_header())
        .send()
        .await?;

    if server_response.status() != UPDATED {
        return Err(Error::BadResponse(server_response.text().await?));
    }

    println!("GitHub automation workflow: {state}d");
    Ok(())
}

pub(crate) async fn set_repository_variable(name: &str, value: &str) -> Result<(), Error> {
    let build = |request: reqwest::RequestBuilder| {
        request
            .headers(git_header())
            .json(&serde_json::json!({
                "name": name,
                "value": value,
            }))
            .send()
    };
    let update_request = reqwest::Client::new().patch(repository_variable_endpoint(name));
    let update_response = build(update_request).await?;

    if update_response.status() == UPDATED {
        println!("Repository variable: {name}, updated");
        return Ok(());
    }

    let create_request = reqwest::Client::new().post(repository_variables_endpoint());
    let create_response = build(create_request).await?;

    if create_response.status() == CREATED {
        println!("Repository variable: {name}, created");
        return Ok(());
    }

    Err(Error::BadResponse(create_response.text().await?))
}

pub(crate) async fn create_remote(content: String) -> Result<GistResponse, Error> {
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

    if server_response.status() != CREATED {
        return Err(Error::BadResponse(server_response.text().await?));
    }

    println!("New private gist created with name: {GIST_NAME}");

    server_response
        .json::<GistResponse>()
        .await
        .map_err(Error::from)
}

pub(crate) async fn update_remote(
    gist_endpoint: &str,
    content: String,
) -> Result<GistResponse, Error> {
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

    if server_response.status() != OK {
        return Err(Error::BadResponse(server_response.text().await?));
    }

    println!("Remote gist successfully updated");

    server_response
        .json::<GistResponse>()
        .await
        .map_err(Error::from)
}

pub(crate) async fn get_remote(gist_endpoint: &str) -> Result<GistResponse, Error> {
    let server_response = reqwest::Client::new()
        .get(gist_endpoint)
        .headers(git_header())
        .send()
        .await?;

    if server_response.status() != OK {
        return Err(Error::BadResponse(server_response.text().await?));
    }

    server_response
        .json::<GistResponse>()
        .await
        .map_err(Error::from)
}

pub(crate) async fn get_public_key() -> Result<RepositoryPublicKey, Error> {
    let server_response = reqwest::Client::new()
        .get(repository_public_key_endpoint())
        .headers(git_header())
        .send()
        .await?;

    if server_response.status() != OK {
        return Err(Error::BadResponse(server_response.text().await?));
    }

    server_response
        .json::<RepositoryPublicKey>()
        .await
        .map_err(Error::from)
}

pub(crate) async fn set_repository_secret(
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

    let print_status = |status: &str| println!("Repository secret: {secret_name}, {status}");

    match server_response.status() {
        CREATED => print_status("created"),
        UPDATED => print_status("updated"),
        _ => return Err(Error::BadResponse(server_response.text().await?)),
    }

    Ok(())
}

fn encrypt_secret(secret: &str, public_key: &str) -> Result<String, Error> {
    let public_key = PublicKey::from_slice(&BASE64.decode(public_key)?).map_err(|err| {
        io::Error::new(ErrorKind::InvalidData, format!("Invalid public key. {err}"))
    })?;

    let encrypted_bytes = public_key.seal(&mut OsRng, secret.as_bytes())?;

    Ok(BASE64.encode(&encrypted_bytes))
}

pub(crate) async fn delete_cache_by_key(key: &str) -> Result<(), Error> {
    let server_response = reqwest::Client::new()
        .delete(repository_cache_endpoint(key))
        .headers(git_header())
        .send()
        .await?;

    if server_response.status() != OK {
        return Err(Error::BadResponse(server_response.text().await?));
    }

    println!("Removed old cache with key: {key}");
    Ok(())
}
