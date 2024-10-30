use crate::{
    check_program_version, conditional_join,
    models::{
        badge_options::BadgePreferences,
        cli::{Mod, SetArgs, Workflow},
        error::Error,
        json_data::Input,
    },
    read,
    services::{
        git::{
            create_remote, delete_cache_by_key, get_public_key, set_repository_secret,
            set_repository_variable, set_workflow_state, update_remote,
        },
        nexus::update_download_counts,
    },
    verify_gist, verify_git, verify_repo, verify_repo_from, write, write_badges, StartupVars,
    ENV_NAME_GIST_ID, ENV_NAME_GIT, ENV_NAME_MODS, ENV_NAME_NEXUS, INPUT_PATH, PREFERENCES_PATH,
    VARS,
};
use std::io::{self, ErrorKind};

pub async fn version(on_remote: bool) -> reqwest::Result<()> {
    let ver_res = check_program_version().await;
    if on_remote {
        match ver_res {
            Ok(Some(_)) => {
                println!("New Nexus Badges version available");
                std::process::exit(70)
            }
            Ok(None) => {
                println!("Nexus Badges up to date");
                std::process::exit(0)
            }
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

trait Update {
    fn write_and_try_set_remote(
        self,
    ) -> impl std::future::Future<Output = Result<(), Error>> + Send;
}

impl Update for Vec<Mod> {
    async fn write_and_try_set_remote(self) -> Result<(), Error> {
        let new_mod_json = (verify_repo().is_ok()).then(|| {
            serde_json::to_string(&self.clone()).expect("`Vec<Mod>` is always ok to stringify")
        });
        let updated = Input::from(VARS.get().expect("set on startup"), self);
        write(updated, INPUT_PATH)?;

        if let Some(new_variable) = new_mod_json {
            if let Err(err) = set_repository_variable(ENV_NAME_MODS, &new_variable).await {
                println!("{INPUT_PATH} updated locally");
                return Err(err);
            }
        }

        Ok(())
    }
}

impl Modify for Vec<Mod> {
    async fn add_mod(mut self, details: Mod) -> Result<(), Error> {
        if self.contains(&details) {
            return Err(Error::Io(io::Error::new(
                ErrorKind::InvalidInput,
                format!("Mod already exists in: {INPUT_PATH}"),
            )));
        }
        self.push(details);
        self.write_and_try_set_remote().await?;

        println!("Mod Registered!");
        Ok(())
    }

    async fn remove_mod(mut self, details: Mod) -> Result<(), Error> {
        let i = self
            .iter()
            .position(|mod_details| *mod_details == details)
            .ok_or_else(|| {
                Error::Io(io::Error::new(
                    ErrorKind::InvalidInput,
                    format!("Mod does not exist in: {INPUT_PATH}"),
                ))
            })?;
        self.swap_remove(i);
        self.write_and_try_set_remote().await?;

        println!("Mod removed!");
        Ok(())
    }
}

macro_rules! propagate_err {
    ($option_res:expr) => {
        if let Some(res) = $option_res {
            res?;
        }
    };
}

pub async fn update_args_local(new: &mut SetArgs) -> Result<(), Error> {
    let mut curr = Input::from_file()?;
    let mut curr_badge = read::<BadgePreferences>(PREFERENCES_PATH).unwrap_or_default();

    if let Some(ref mut token) = new.git {
        new.modified.git_token = true;
        curr.git_token = std::mem::take(token);
    }
    if let Some(ref mut key) = new.nexus {
        new.modified.nexus_key = true;
        curr.nexus_key = std::mem::take(key);
    }
    if let Some(ref mut id) = new.gist {
        new.modified.gist_id = true;
        std::mem::swap(&mut curr.gist_id, id);
    }
    if let Some(ref mut repo) = new.repo {
        curr.repo = std::mem::take(repo);
    }
    if let Some(ref mut owner) = new.owner {
        curr.owner = std::mem::take(owner);
    }

    if let Some(style) = new.style {
        curr_badge.set_style(style);
    }
    if let Some(counter) = new.counter {
        curr_badge.counter = counter;
    }
    if let Some(ref mut label) = new.label {
        curr_badge.label = std::mem::take(label);
    }
    if let Some(color) = new.label_color {
        curr_badge.label_color = color;
    }
    if let Some(color) = new.color {
        curr_badge.color = color;
    }

    let return_res = verify_repo_from(&curr.owner, &curr.repo);

    if new.key_modified() {
        write(curr, INPUT_PATH)?;

        if let Some(ref prev_id) = new.gist {
            if !prev_id.is_empty() {
                // MARK: XXX
                // Do we require confirmation for these kind of overwrites?
                println!("WARN: Previously stored gist_id: {prev_id}, was replaced");
            }
        }

        println!("Key(s) updated locally");
    }

    if new.pref_modified() {
        write(curr_badge, PREFERENCES_PATH)?;
        println!("Badge preference(s) updated")
    }

    return_res
}

pub async fn update_args_remote(new: SetArgs) -> Result<(), Error> {
    debug_assert!(
        verify_repo().is_ok(),
        "expects condtion is checked before this fn is called"
    );

    let vars = VARS.get().expect("set on startup");

    let public_key_task = (new.modified.git_token || new.modified.nexus_key).then(get_public_key);
    let set_gist_id_task = new
        .modified
        .gist_id
        .then(|| set_repository_variable(ENV_NAME_GIST_ID, &vars.gist_id));

    let (public_key_res, set_gist_id_res) =
        conditional_join(public_key_task, set_gist_id_task).await;

    if let Some(res) = public_key_res {
        let public_key = res?;

        let set_git_token_task = new
            .modified
            .git_token
            .then(|| set_repository_secret(ENV_NAME_GIT, &vars.git_token, &public_key));
        let set_nexus_key_task = new
            .modified
            .nexus_key
            .then(|| set_repository_secret(ENV_NAME_NEXUS, &vars.nexus_key, &public_key));

        let (set_git_token_res, set_nexus_key_res) =
            conditional_join(set_git_token_task, set_nexus_key_task).await;

        propagate_err!(set_git_token_res);
        propagate_err!(set_nexus_key_res);
    }

    propagate_err!(set_gist_id_res);

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
        write_badges(output, prev_remote.universal_url()?)?;
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

/// NOTE: this command is not supported on local
pub async fn update_cache_key(old: Option<&str>, new: &str) -> Result<(), Error> {
    const CACHE_KEY: &str = "CACHED_BIN";

    VARS.set(StartupVars::git_api_only()?)
        .expect("`startup` never gets to run");

    let delete_task = old.map(delete_cache_by_key);

    let (delete_res, set_res) =
        conditional_join(delete_task, Some(set_repository_variable(CACHE_KEY, new))).await;

    propagate_err!(set_res);
    propagate_err!(delete_res);

    Ok(())
}
