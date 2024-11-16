use std::{path::PathBuf, str::FromStr};

use eyre::{Context as _, ContextCompat as _};
use gix::bstr::ByteSlice;

#[derive(clap::Parser)]
pub enum Args {
    Import { path: PathBuf },
    Clone { repo: RepoUrl },
    Sync { endpoints: Vec<String> },
}

#[derive(Clone, Debug)]
pub struct RepoUrl {
    url: gix::Url,
    pub host: String,
    pub repo_name: String,
    pub owner: String,
}

impl RepoUrl {
    pub fn as_url(&self) -> &gix::Url {
        &self.url
    }

    pub fn from_url(url: gix::Url) -> Result<Self, eyre::Error> {
        let host = url.host().context("Missing hostname")?.to_owned();
        let path = url.path.to_str()?;
        let mut path_segments = path.split("/");
        let owner = path_segments.next().context("Missing owner")?.to_owned();
        let repo_name = path_segments.next().context("Missing repo name")?;

        let repo_name = repo_name
            .strip_suffix(".git")
            .unwrap_or(repo_name)
            .to_owned();

        Ok(RepoUrl {
            url,
            host,
            repo_name,
            owner,
        })
    }
}

impl FromStr for RepoUrl {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url = gix::Url::from_bytes(s.as_bytes().into()).context("Invalid git repo ref")?;
        Self::from_url(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn git_scheme_works() {
        let parsed = RepoUrl::from_str("git@github.com:foldu/src-manage.git").unwrap();

        assert_eq!(&parsed.host, "github.com");
        assert_eq!(&parsed.owner, "foldu");
        assert_eq!(&parsed.repo_name, "src-manage");
    }
}
