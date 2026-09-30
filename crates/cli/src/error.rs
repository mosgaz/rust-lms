use std::io;

pub type CliResult<T> = Result<T, CliError>;

#[derive(Debug)]
pub enum CliError {
    Io(io::Error),
    Prompt(dialoguer::Error),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::Io(e) => write!(f, "io error: {e}"),
            CliError::Prompt(e) => write!(f, "prompt error: {e}"),
        }
    }
}

impl std::error::Error for CliError {}

impl From<io::Error> for CliError {
    fn from(e: io::Error) -> Self {
        CliError::Io(e)
    }
}

impl From<dialoguer::Error> for CliError {
    fn from(e: dialoguer::Error) -> Self {
        CliError::Prompt(e)
    }
}
