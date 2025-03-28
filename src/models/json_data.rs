use crate::models::cli::Mod;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Input {
    pub git_token: String,
    pub nexus_key: String,
    pub gist_id: String,
    pub owner: String,
    pub repo: String,
    pub mods: Vec<Mod>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct ModDetails {
    pub name: String,
    #[serde(skip_deserializing)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub url: String,
    #[serde(skip_serializing)]
    pub uid: u64,
    pub mod_downloads: usize,
    pub mod_unique_downloads: usize,
}

#[derive(Deserialize)]
pub(crate) struct GistResponse {
    pub id: String,
    pub files: HashMap<String, FileDetails>,
}

#[derive(Deserialize)]
pub(crate) struct FileDetails {
    pub raw_url: String,
    pub content: String,
}

#[derive(Deserialize)]
pub(crate) struct Version {
    pub latest: String,
    pub message: String,
}

#[derive(Deserialize)]
pub(crate) struct RepositoryPublicKey {
    pub key_id: String,
    pub key: String,
}
