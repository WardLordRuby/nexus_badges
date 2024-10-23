use crate::cli::Mod;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Default, Clone)]
#[serde(default)]
pub struct Input {
    pub git_token: String,
    pub nexus_key: String,
    pub gist_id: String,
    pub owner: String,
    pub repo: String,
    pub mods: Vec<Mod>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ModDetails {
    pub name: String,
    #[serde(skip_deserializing)]
    pub url: String,
    #[serde(skip_serializing)]
    pub uid: u64,
    pub mod_downloads: usize,
    pub mod_unique_downloads: usize,
}

#[derive(Deserialize)]
pub struct GistResponse {
    pub id: String,
    pub files: HashMap<String, FileDetails>,
}

#[derive(Deserialize)]
pub struct FileDetails {
    pub raw_url: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct Version {
    pub latest: String,
    pub message: String,
}

#[derive(Deserialize)]
pub struct RepositoryPublicKey {
    pub key_id: String,
    pub key: String,
}
