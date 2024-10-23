use crate::{
    models::{
        cli::{Mod, SetArgs},
        error::Error,
        json_data::Input,
    },
    services::{
        git::{create_remote, get_public_key, set_repository_secret, update_remote},
        nexus::update_download_counts,
    },
    verify_gist, verify_git, verify_repo, write, write_badges, ENV_NAME_GIT, ENV_NAME_NEXUS,
    INPUT_PATH, VARS,
};
use std::{
    io::{self, ErrorKind},
    sync::Arc,
};
use tokio::task::JoinSet;

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
            let public_key = get_public_key().await.map_or_else(
                |err| {
                    eprintln!("{err}");
                    None
                },
                Some,
            );

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
                if let Err(err) = res.expect("task can not panic") {
                    eprintln!("{err}")
                }
            }
        }

        Ok(())
    }
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

    let mut meta = create_remote(serde_json::to_string_pretty(&output)?).await?;

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
