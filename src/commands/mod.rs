mod disable;
mod enable;
mod start;

/// Represents a subcommand handler.
pub trait Run {
    fn run(&self) -> anyhow::Result<()>;
}

#[derive(Debug, clap::Parser)]
pub struct Parser {
    #[clap(subcommand)]
    pub subcommand: Subcommand,
}

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    /// Run the automatic theme switcher.
    Start(start::Start),
    /// If disabled, start the automatic theme switcher again.
    Enable(enable::Enable),
    /// If enabled, stop the automatic theme switcher.
    Disable(disable::Disable),
}

impl Subcommand {
    pub fn run(&self) -> anyhow::Result<()> {
        let handler: &dyn Run = match *self {
            Self::Start(ref inner) => inner,
            Self::Enable(ref inner) => inner,
            Self::Disable(ref inner) => inner,
        };

        handler.run()
    }
}
