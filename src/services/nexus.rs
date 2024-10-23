use crate::{
    models::{cli::Mod, error::Error, json_data::ModDetails},
    verify_added, verify_nexus, write, INPUT_PATH, OUPUT_PATH, VARS,
};
use std::{
    collections::BTreeMap,
    io::{self, ErrorKind},
};
use tokio::task::JoinSet;

const NEXUS_BASE_URL: &str = "https://api.nexusmods.com";

const NEXUS_INFO_OK: u16 = 200;

impl Mod {
    fn get_info_endpoint(&self) -> String {
        format!(
            "{NEXUS_BASE_URL}/v1/games/{}/mods/{}.json",
            self.domain, self.mod_id
        )
    }
    pub fn url(&self) -> String {
        format!(
            "https://www.nexusmods.com/{}/mods/{}",
            self.domain, self.mod_id
        )
    }
}

pub async fn update_download_counts(mods: Vec<Mod>) -> Result<BTreeMap<u64, ModDetails>, Error> {
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
