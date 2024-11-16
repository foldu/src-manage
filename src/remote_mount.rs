use std::{ffi::OsString, path::Path, process::Command};

use tempfile::TempDir;

use crate::command_ext::{self, CommandExt};

pub struct TempMount {
    dir: TempDir,
}

impl TempMount {
    pub fn path(&self) -> &Path {
        self.dir.path()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed running command`")]
    Command {
        #[from]
        err: command_ext::Error,
    },
    #[error("Failed creating temporary directory")]
    TempDir { source: std::io::Error },
}

pub fn temp_mount(host: &str, path: impl AsRef<Path>) -> Result<TempMount, Error> {
    let path = path.as_ref();
    let mut connection_string = OsString::new();
    connection_string.push(host);
    connection_string.push(":");
    connection_string.push(path);

    // NOTE: TempDir::new creates dirs only user read/writeable so no need to customize permissions
    let tmp_dir = TempDir::new().map_err(|e| Error::TempDir { source: e })?;
    Command::new("sshfs")
        .arg(connection_string)
        .arg(tmp_dir.path())
        .run()?;
    Ok(TempMount { dir: tmp_dir })
}

impl Drop for TempMount {
    fn drop(&mut self) {
        Command::new("fusermount")
            .arg("-u")
            .arg(self.dir.path())
            .run()
            .unwrap();
    }
}
