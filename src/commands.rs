use crate::{
    check_program_version,
    models::{
        cli::{Mod, SetArgs, Workflow},
        error::Error,
        json_data::Input,
    },
    services::{
        git::{
            create_remote, get_public_key, set_repository_secret, set_repository_variable,
            set_workflow_state, update_remote,
        },
        nexus::update_download_counts,
    },
    verify_gist, verify_git, verify_repo, write, write_badges, ENV_NAME_GIST_ID, ENV_NAME_GIT,
    ENV_NAME_MODS, ENV_NAME_NEXUS, INPUT_PATH, VARS,
};
use std::{
    io::{self, ErrorKind},
    sync::Arc,
};
use tokio::task::JoinSet;

pub async fn version(on_remote: bool) -> reqwest::Result<()> {
    let ver_res = check_program_version().await;
    if on_remote {
        match ver_res {
            Ok(Some(_)) => std::process::exit(70),
            Ok(None) => std::process::exit(0),
            Err(err) => {
                eprintln!("{err}");
                std::process::exit(20)
            }
        }
    }
    println!("nexus_mods v{}", env!("CARGO_PKG_VERSION"));
    if let Some(msg) = ver_res? {
        println!("{msg}")
    }
    Ok(())
}

pub trait Modify {
    fn add_mod(self, details: Mod) -> impl std::future::Future<Output = Result<(), Error>> + Send;
    fn remove_mod(
        self,
        details: Mod,
    ) -> impl std::future::Future<Output = Result<(), Error>> + Send;
}

async fn update_mods_in_input(input_mods: Vec<Mod>) -> Result<(), Error> {
    let new_mod_json = (verify_repo().is_ok()).then(|| {
        serde_json::to_string(&input_mods.clone()).expect("`Vec<Mod>` is always ok to stringify")
    });
    let updated = Input::from(VARS.get().expect("set on startup"), input_mods);
    write(updated, INPUT_PATH)?;

    if let Some(new_variable) = new_mod_json {
        if let Err(err) = set_repository_variable(ENV_NAME_MODS, &new_variable).await {
            println!("{INPUT_PATH} updated locally");
            return Err(err);
        }
    }

    Ok(())
}

impl Modify for Vec<Mod> {
    async fn add_mod(mut self, details: Mod) -> Result<(), Error> {
        if self.contains(&details) {
            Err(Error::Io(io::Error::new(
                ErrorKind::InvalidInput,
                format!("Mod already exists in: {INPUT_PATH}"),
            )))
        } else {
            self.push(details);

            update_mods_in_input(self).await?;

            println!("Mod Registered!");
            Ok(())
        }
    }

    async fn remove_mod(mut self, details: Mod) -> Result<(), Error> {
        if let Some(i) = self.iter().position(|mod_details| *mod_details == details) {
            self.swap_remove(i);

            update_mods_in_input(self).await?;

            println!("Mod removed!");
            Ok(())
        } else {
            Err(Error::Io(io::Error::new(
                ErrorKind::InvalidInput,
                format!("Mod does not exist in: {INPUT_PATH}"),
            )))
        }
    }
}

macro_rules! clone_if {
    ($condition:expr, $target:ident, $value:expr) => {
        if $condition {
            $target = Some($value.clone());
        }
    };
}

pub async fn update_args(input_mods: Vec<Mod>, mut new: SetArgs) -> Result<(), Error> {
    let mut curr = Input::from(VARS.get().expect("set on startup"), input_mods);
    let try_update_remote_env = verify_repo().is_ok();
    let (mut new_git_token, mut new_nexus_key, mut new_gist_id) = (None, None, None);

    if let Some(token) = new.git {
        clone_if!(try_update_remote_env, new_git_token, token);
        curr.git_token = token;
    }
    if let Some(key) = new.nexus {
        clone_if!(try_update_remote_env, new_nexus_key, key);
        curr.nexus_key = key;
    }
    if let Some(ref mut id) = new.gist {
        clone_if!(try_update_remote_env, new_gist_id, id);
        std::mem::swap(&mut curr.gist_id, id);
    }
    if let Some(repo) = new.repo {
        curr.repo = repo;
    }
    if let Some(owner) = new.owner {
        curr.owner = owner;
    }

    write(curr, INPUT_PATH)?;

    if let Some(prev_id) = new.gist {
        if !prev_id.is_empty() {
            // MARK: XXX
            // Do we require confirmation for these kind of overwrites?
            println!("WARN: Previously stored gist_id: {prev_id}, was replaced");
        }
    }

    println!("Key(s) updated locally");

    if try_update_remote_env
        && (new_git_token.is_some() || new_nexus_key.is_some() || new_gist_id.is_some())
    {
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
        if let Some(id) = new_gist_id {
            tasks.spawn(async move { set_repository_variable(ENV_NAME_GIST_ID, &id).await });
        }

        while let Some(res) = tasks.join_next().await {
            if let Err(err) = res.expect("task can not panic") {
                eprintln!("{err}")
            }
        }
    }

    Ok(())
}

pub async fn process(input_mods: Vec<Mod>, on_remote: bool) -> Result<(), Error> {
    let (output_res, verify_res) =
        tokio::join!(update_download_counts(input_mods, on_remote), verify_gist());

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

    if !on_remote {
        let universal_url = prev_remote.universal_url()?;
        write_badges(output, universal_url)?;
    }
    Ok(())
}

pub async fn init_remote(input_mods: Vec<Mod>) -> Result<(), Error> {
    verify_git()?;
    let mut input = Input::from(VARS.get().expect("set on startup"), input_mods.clone());
    let output = update_download_counts(input_mods, false).await?;

    let mut meta = create_remote(serde_json::to_string_pretty(&output)?).await?;

    let swapped_old = !input.gist_id.is_empty() && input.gist_id != meta.id;

    println!("New gist_id: {}", meta.id);

    std::mem::swap(&mut input.gist_id, &mut meta.id);
    write(input, INPUT_PATH)?;

    if swapped_old {
        println!("Previous gist_id: {}, was replaced", meta.id);
    }

    write_badges(output, meta.universal_url()?)?;

    Ok(())
}

pub async fn init_actions(input_mods: Vec<Mod>) -> Result<(), Error> {
    update_remote_variables(input_mods).await?;
    set_workflow_state(Workflow::Enable).await?;
    Ok(())
}

async fn update_remote_variables(input_mods: Vec<Mod>) -> Result<(), Error> {
    verify_repo()?;

    let vars = VARS.get().expect("set on startup");
    let mods_str =
        serde_json::to_string(&input_mods).expect("`Vec<Mod>` is always ok to stringify");
    let (public_key_res, gist_id_res, input_mods_res) = tokio::join!(
        get_public_key(),
        set_repository_variable(ENV_NAME_GIST_ID, &vars.gist_id),
        set_repository_variable(ENV_NAME_MODS, &mods_str)
    );

    gist_id_res?;
    input_mods_res?;
    let public_key = public_key_res?;

    let (git_secret_res, nexus_secret_res) = tokio::join!(
        set_repository_secret(ENV_NAME_GIT, &vars.git_token, &public_key),
        set_repository_secret(ENV_NAME_NEXUS, &vars.nexus_key, &public_key)
    );

    git_secret_res?;
    nexus_secret_res?;

    Ok(())
}
