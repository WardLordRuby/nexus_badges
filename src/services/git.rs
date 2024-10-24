use crate::{
    models::{
        error::Error,
        json_data::{FileDetails, GistResponse, RepositoryPublicKey},
    },
    CREATED_RESPONSE, OK_RESPONSE, UPDATED_RESPONSE, VARS,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use crypto_box::{aead::OsRng, PublicKey};
use reqwest::header::{HeaderMap, HeaderValue};
use std::{
    borrow::Cow,
    io::{self, ErrorKind},
};

const GIT_BASE_URL: &str = "https://api.github.com";
const GIT_API_VER: &str = "2022-11-28";

const GIST_NAME: &str = "nexus_badges.json";
const GIST_DESC: &str = "Private gist to be used as a json endpoint for badge download counters";

const RAW: &str = "/raw/";

impl GistResponse {
    fn file_details(&self) -> Result<&FileDetails, Error> {
        self.files.get(GIST_NAME).ok_or_else(|| {
            Error::BadResponse(format!(
                "Gist response did not contains details about any file with the name: {GIST_NAME}"
            ))
        })
    }

    pub fn universal_url(&self) -> Result<&str, Error> {
        self.file_details().map(|entry| {
            let i = entry.raw_url.find(RAW).expect("always contains `RAW`");
            &entry.raw_url[..i + RAW.len()]
        })
    }

    pub fn content(&self) -> Result<&str, Error> {
        self.file_details().map(|entry| entry.content.as_str())
    }
}

pub fn gist_id_endpoint() -> String {
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

pub async fn set_repository_variable(name: &str, value: &str) -> Result<(), Error> {
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

    if update_response.status() != UPDATED_RESPONSE {
        let create_request = reqwest::Client::new().post(repository_variables_endpoint());
        let create_response = build(create_request).await?;

        if create_response.status() != CREATED_RESPONSE {
            return Err(Error::BadResponse(create_response.text().await?));
        }

        println!("Repository variable: {name}, created");
        return Ok(());
    }

    println!("Repository variable: {name}, updated");
    Ok(())
}

pub async fn create_remote(content: String) -> Result<GistResponse, Error> {
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

    if server_response.status() != CREATED_RESPONSE {
        return Err(Error::BadResponse(server_response.text().await?));
    }

    println!("New private gist created with name: {GIST_NAME}");

    server_response
        .json::<GistResponse>()
        .await
        .map_err(Error::from)
}

pub async fn update_remote(gist_endpoint: &str, content: String) -> Result<GistResponse, Error> {
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

    if server_response.status() != OK_RESPONSE {
        return Err(Error::BadResponse(server_response.text().await?));
    }

    println!("Remote gist successfully updated");

    server_response
        .json::<GistResponse>()
        .await
        .map_err(Error::from)
}

pub async fn get_remote(gist_endpoint: &str) -> Result<GistResponse, Error> {
    let server_response = reqwest::Client::new()
        .get(gist_endpoint)
        .headers(git_header())
        .send()
        .await?;

    if server_response.status() != OK_RESPONSE {
        return Err(Error::BadResponse(server_response.text().await?));
    }

    server_response
        .json::<GistResponse>()
        .await
        .map_err(Error::from)
}

pub async fn get_public_key() -> Result<RepositoryPublicKey, Error> {
    let server_response = reqwest::Client::new()
        .get(repository_public_key_endpoint())
        .headers(git_header())
        .send()
        .await?;

    if server_response.status() != OK_RESPONSE {
        return Err(Error::BadResponse(server_response.text().await?));
    }

    server_response
        .json::<RepositoryPublicKey>()
        .await
        .map_err(Error::from)
}

pub async fn set_repository_secret(
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

    if server_response.status() == CREATED_RESPONSE || server_response.status() == UPDATED_RESPONSE
    {
        println!(
            "Repository secret: {secret_name}, {}",
            match server_response.status() {
                s if s == CREATED_RESPONSE => "created",
                s if s == UPDATED_RESPONSE => "updated",
                _ => unreachable!("by outer if"),
            }
        );
        return Ok(());
    }

    Err(Error::BadResponse(server_response.text().await?))
}

fn encrypt_secret(secret: &str, public_key: &str) -> Result<String, Error> {
    let public_key = PublicKey::from_slice(&BASE64.decode(public_key)?).map_err(|err| {
        io::Error::new(ErrorKind::InvalidData, format!("Invalid public key. {err}"))
    })?;

    let encrypted_bytes = public_key.seal(&mut OsRng, secret.as_bytes())?;

    Ok(BASE64.encode(&encrypted_bytes))
}
