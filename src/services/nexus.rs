use crate::{
    models::{cli::Mod, error::Error, json_data::ModDetails},
    verify_added, verify_nexus, write, OK_RESPONSE, PATHS, TOTAL_KEY, VARS,
};
use std::{
    collections::BTreeMap,
    io::{self, ErrorKind},
};
use tokio::task::JoinSet;

const NEXUS_BASE_URL: &str = "https://api.nexusmods.com";

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

async fn abort_and_wait<T: 'static>(tasks: &mut JoinSet<T>) {
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
}

pub async fn update_download_counts(
    mods: Vec<Mod>,
    on_remote: bool,
) -> Result<BTreeMap<String, ModDetails>, Error> {
    verify_nexus()?;
    verify_added(&mods)?;

    let client = reqwest::Client::new();
    let mut tasks = JoinSet::new();
    let mut total = ModDetails::total();

    for descriptor in mods.into_iter() {
        tasks.spawn(try_get_info(descriptor, client.clone()));
    }

    let mut output = BTreeMap::new();

    while let Some(res) = tasks.join_next().await {
        match res {
            Ok(Ok(data)) => {
                total.add(&data);
                if let Some(dup) = output.insert(data.uid.to_string(), data) {
                    abort_and_wait(&mut tasks).await;
                    return Err(Error::Io(io::Error::new(
                        ErrorKind::InvalidInput,
                        format!("duplicate tracked mod: {}, in: {}", dup.name, PATHS.input),
                    )));
                }
            }
            Ok(Err(err)) => {
                abort_and_wait(&mut tasks).await;
                return Err(err);
            }
            Err(err) => {
                abort_and_wait(&mut tasks).await;
                return Err(err.into());
            }
        }
    }

    output.insert(TOTAL_KEY.to_string(), total);

    println!("Retrieved download counts from Nexus Mods");

    if !on_remote {
        write(output.clone(), &PATHS.output)?;
        println!(
            "Download counts saved locally for {} mod(s)",
            output.len() - 1
        );
    }

    Ok(output)
}

async fn try_get_info(details: Mod, client: reqwest::Client) -> Result<ModDetails, Error> {
    let server_response = client
        .get(details.get_info_endpoint())
        .header("accept", "application/json")
        .header("apikey", &VARS.get().expect("set on startup").nexus_key)
        .send()
        .await?;

    if server_response.status() != OK_RESPONSE {
        return Err(Error::BadResponse(server_response.text().await?));
    }

    server_response
        .json::<ModDetails>()
        .await
        .map(|output| output.add_url(&details))
        .map_err(Error::from)
}
