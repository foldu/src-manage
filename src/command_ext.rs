use std::process::Command;

pub trait CommandExt {
    fn run(&mut self) -> Result<(), Error>;
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Missing executable `{name}`")]
    MissingProgram { name: String },
    #[error("Failed executing {cmd}")]
    ProgramFailed { cmd: String },
}

impl CommandExt for &mut Command {
    fn run(&mut self) -> Result<(), Error> {
        let ret = self.status().map_err(|_| Error::MissingProgram {
            name: self.get_program().to_str().unwrap().to_owned(),
        })?;

        if !ret.success() {
            return Err(Error::ProgramFailed {
                cmd: format!("{:?}", self),
            });
        }

        Ok(())
    }
}
