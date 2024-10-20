use crate::{cli::Mod, GIST_NAME};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct Input {
    pub git_token: String,
    pub nexus_key: String,
    pub gist_id: String,
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

#[derive(Deserialize, Serialize)]
pub struct GistNew {
    description: String,
    public: bool,
    files: HashMap<String, Upload>,
}

#[derive(Deserialize, Serialize)]
pub struct GistUpdate {
    files: HashMap<String, Upload>,
}

#[derive(Deserialize, Serialize)]
pub struct Upload {
    content: String,
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

impl From<Upload> for GistNew {
    fn from(value: Upload) -> Self {
        GistNew {
            description: String::from(
                "Private gist to be used as a json endpoint for badge download counters",
            ),
            public: false,
            files: HashMap::from([(GIST_NAME.to_string(), value)]),
        }
    }
}

impl From<Upload> for GistUpdate {
    fn from(value: Upload) -> Self {
        GistUpdate {
            files: HashMap::from([(GIST_NAME.to_string(), value)]),
        }
    }
}

impl From<String> for Upload {
    fn from(value: String) -> Self {
        Upload { content: value }
    }
}

#[derive(Deserialize)]
pub struct Version {
    pub latest: String,
    pub message: String,
}
