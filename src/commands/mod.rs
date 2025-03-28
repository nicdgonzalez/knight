mod disable;
mod enable;
mod run;
mod set;

#[derive(Debug, clap::Subcommand)]
pub enum Command {
    /// If disabled, re-enable automatic theme switching.
    Enable,
    /// Pause automatic theme switching indefinitely.
    Disable,
    /// Manually set the theme for today.
    Set(set::Args),
    /// Switch the system to the appropriate theme.
    Run,
}

pub fn handle_command(command: &Command) -> Result<(), Error> {
    match &command {
        Command::Enable => enable::run(),
        Command::Disable => disable::run(),
        Command::Set(args) => set::run(args),
        Command::Run => run::run(),
    }
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    FailedToDeserialize(toml::de::Error),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(inner) => write!(f, "{}", inner),
            Self::FailedToDeserialize(inner) => write!(f, "{}", inner),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<toml::de::Error> for Error {
    fn from(value: toml::de::Error) -> Self {
        Self::FailedToDeserialize(value)
    }
}
