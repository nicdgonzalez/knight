#![warn(
    clippy::correctness,
    clippy::suspicious,
    clippy::complexity,
    clippy::perf,
    clippy::style,
    clippy::pedantic
)]

mod cache;
mod commands;
mod config;
mod daylight;
mod persistent;
mod state;
mod theme;

use std::io;
use std::io::Write as _;
use std::process::ExitCode;

use clap::Parser as _;
use colored::Colorize as _;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::prelude::*;

fn main() -> ExitCode {
    try_main().unwrap_or_else(|err| {
        let mut stderr = io::stderr().lock();
        _ = writeln!(stderr, "{}", "knight failed".bold().red());

        for cause in err.chain() {
            _ = writeln!(stderr, "  {}: {}", "Cause".bold(), cause);
        }

        ExitCode::FAILURE
    })
}

fn try_main() -> anyhow::Result<ExitCode> {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=trace", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = commands::Parser::parse();
    args.subcommand.run().map(|()| ExitCode::SUCCESS)
}
