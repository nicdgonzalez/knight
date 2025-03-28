use clap::Parser;

use knight::commands;
use knight::config::Configuration;

/// 🛡️ Automatically switch the system between light and dark theme.
#[derive(Debug, clap::Parser)]
#[command(
    version,
    after_help = "Repository: https://github.com/nicdgonzalez/knight"
)]
pub struct Args {
    #[command(subcommand)]
    pub command: commands::Command,
}

fn main() {
    if let Err(err) = try_main() {
        eprintln!("error: {:#?}", err);
        std::process::exit(1);
    }
}

fn try_main() -> Result<(), commands::Error> {
    let content = include_str!("../Knight.toml");
    let _config: Configuration = toml::from_str(&content).unwrap();

    let args = Args::parse();
    commands::handle_command(&args.command)
}
