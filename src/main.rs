use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{atomic::AtomicBool, Arc, LazyLock},
};

use args::RepoUrl;
use clap::Parser;
use command_ext::CommandExt as _;
use config::Config;
use etcetera::{AppStrategy, AppStrategyArgs};
use eyre::{Context as _, ContextCompat as _};
use gix::{bstr::BString, remote::Direction, status::UntrackedFiles};

mod args;
mod command_ext;
mod config;
mod remote_mount;

static SHOULD_STOP: LazyLock<Arc<AtomicBool>> = LazyLock::new(|| Arc::new(AtomicBool::new(false)));

fn main() -> Result<(), eyre::Error> {
    let strategy = etcetera::choose_app_strategy(AppStrategyArgs {
        top_level_domain: "li".to_string(),
        author: "5kw".to_string(),
        app_name: env!("CARGO_PKG_NAME").to_string(),
    })?;
    let cfg_path = strategy.config_dir().join("config.json");

    let config = Config::from_path(cfg_path)?;
    signal_hook::flag::register(signal_hook::consts::SIGTERM, SHOULD_STOP.clone())?;
    signal_hook::flag::register(signal_hook::consts::SIGINT, SHOULD_STOP.clone())?;

    match args::Args::parse() {
        args::Args::Import { path } => {
            import(&config, path.clone())
                .with_context(|| format!("Failed importing path {}", path.display()))?;
        }
        args::Args::Clone { repo } => {
            let path = config.get_directory_for(&repo);
            clone(&repo, &path).with_context(|| {
                format!("Failed cloning {} to {}", repo.as_url(), path.display())
            })?;
        }
        args::Args::Sync { endpoints } => {
            let endpoints = if endpoints.is_empty() {
                &config.sync_default
            } else {
                &endpoints
            };
            for endpoint in endpoints {
                sync(&config, endpoint)?;
            }
        }
    }

    Ok(())
}

fn sync(config: &Config, host: &str) -> Result<(), eyre::Error> {
    let repo = gix::open(".")?;
    let repo_url = get_repo_url(&repo)?;
    let path = config.get_directory_for(&repo_url);

    let mut has_changes = false;
    match remote_mount::temp_mount(host, &path) {
        Err(
            e @ remote_mount::Error::Command {
                err: command_ext::Error::MissingProgram { .. },
            },
        ) => {
            Err(e)?;
        }
        Err(_) => {}
        Ok(repo) => {
            // TODO: diff between repos to see if there are diverging commits
            let tmp_repo = gix::open(repo.path())?;
            let status = tmp_repo.status(gix::progress::Discard)?;
            // NOTE: Vec::new doesn't allocate so this does nothing but makes type inference
            // in into_index_worktree_iter less annoying
            let filter: Vec<BString> = Vec::new();
            for stat in status
                .should_interrupt_shared(&SHOULD_STOP)
                .untracked_files(UntrackedFiles::None)
                .into_index_worktree_iter(filter)?
            {
                let _stat = stat?;
                has_changes = true;
            }
        }
    }

    if has_changes {
        let abort = dialoguer::Confirm::new()
            .with_prompt("Remote has changes, really clobber?")
            .interact()?;
        if abort {
            return Ok(());
        }
    }

    let mut target = OsString::new();
    target.push(host);
    target.push(":");
    target.push(&path);
    target.push("/");

    Command::new("rsync")
        .arg("--filter=:- .gitignore")
        .arg("-azvP")
        .arg("./")
        .arg(target)
        .run()?;

    Ok(())
}

fn get_repo_url(repo: &gix::Repository) -> Result<RepoUrl, eyre::Error> {
    let remote = repo
        .find_default_remote(Direction::Push)
        .context("Missing default remote")?
        .context("Can't find remote")?;
    let url = remote.url(Direction::Push).context("Missing remote url")?;

    RepoUrl::from_url(url.clone())
}

fn import(config: &Config, repo_path: PathBuf) -> Result<(), eyre::Error> {
    let repo = gix::open(&repo_path)?;
    let remote = get_repo_url(&repo)?;

    let import_dir = config.get_directory_for(&remote);
    if import_dir.exists() {
        eyre::bail!("Already imported to {}", import_dir.display());
    }

    create_parent_if_not_exists(&import_dir)?;

    fs::rename(&repo_path, &import_dir).with_context(|| {
        format!(
            "Failed renaming {} to {}",
            repo_path.display(),
            import_dir.display()
        )
    })?;

    println!(
        "Imported {} to {}",
        repo_path.display(),
        import_dir.display()
    );

    Ok(())
}

fn create_parent_if_not_exists(path: impl AsRef<Path>) -> Result<(), eyre::Error> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed creating directory {}", parent.display()))?
    }

    Ok(())
}

fn clone(repo: &RepoUrl, path: &Path) -> Result<(), eyre::Error> {
    if path.exists() {
        eyre::bail!("Already cloned to {}", path.display());
    }

    create_parent_if_not_exists(path)?;

    let mut progress = gix::progress::Discard;
    let mut prep_fetch = gix::prepare_clone(repo.as_url().clone(), path)?;

    let (mut prep_checkout, _outcome) =
        prep_fetch.fetch_then_checkout(&mut progress, &SHOULD_STOP)?;

    let (_repo, _outcome) = prep_checkout.main_worktree(&mut progress, &SHOULD_STOP)?;
    println!("Cloned {} to {}", repo.as_url(), path.display());
    Ok(())
}
