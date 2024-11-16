use eyre::Context;
use serde::{Deserialize, Deserializer};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use crate::args::RepoUrl;

#[derive(Deserialize)]
pub struct Config {
    pub config: ConfigConfig,
    pub hosts: HashMap<String, HostConfig>,
    #[serde(default)]
    pub sync_default: Vec<String>,
}

impl Config {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Config, eyre::Error> {
        let path = path.as_ref();
        let err = || format!("Failed loading config from {}", path.display());
        let cont = std::fs::read_to_string(path).with_context(err)?;
        Config::from_str(&cont).with_context(err)
    }

    pub fn from_str(s: &str) -> Result<Config, eyre::Error> {
        serde_json::from_str(s).context("Failed deserializing config")
    }

    pub fn get_directory_for(&self, repo: &RepoUrl) -> PathBuf {
        let mut ret = self.config.src_dir.join(&repo.host);
        let add_owner = self
            .hosts
            .get(&repo.host)
            .map(|e| !e.flatten)
            .unwrap_or(true);
        if add_owner {
            ret.push(&repo.owner);
        }
        ret.push(&repo.repo_name);
        ret
    }
}

#[derive(Deserialize)]
pub struct ConfigConfig {
    #[serde(deserialize_with = "deserialize_with_tilde")]
    pub src_dir: PathBuf,
}

fn deserialize_with_tilde<'de, D>(de: D) -> Result<PathBuf, D::Error>
where
    D: Deserializer<'de>,
{
    let path = PathBuf::deserialize(de)?;
    Ok(expanduser(&path))
}

// TODO: support for user name expansion
fn expanduser(path: impl AsRef<Path>) -> PathBuf {
    static HOME_DIR: LazyLock<PathBuf> =
        LazyLock::new(|| etcetera::home_dir().expect("Failed getting home dir"));
    let path = path.as_ref();
    if let Ok(cleaned_path) = path.strip_prefix(Path::new("~/")) {
        HOME_DIR.join(cleaned_path)
    } else if path == Path::new("~") {
        HOME_DIR.to_owned()
    } else {
        path.to_owned()
    }
}

#[derive(Deserialize)]
pub struct HostConfig {
    pub flatten: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expanduser_works() {
        let home = etcetera::home_dir().expect("Failed getting home dir");
        assert_eq!(expanduser("~/test.cfg"), home.join("test.cfg"));

        assert_eq!(expanduser("~"), home);
        assert_eq!(expanduser("lol"), PathBuf::from("lol"))
    }
}
