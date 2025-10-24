use anyhow::Context as _;

use crate::{commands::Run, state::State};

#[derive(Debug, clap::Args)]
pub struct Enable;

impl Run for Enable {
    fn run(&self) -> anyhow::Result<()> {
        let mut state_path = dirs::data_dir().context("failed to get data directory")?;
        state_path.extend(["knight", "state.json"]);
        let state_path = state_path;

        let mut state = State::from_file(&state_path).unwrap_or_default();
        state.disabled = false;
        state.manually_override_until = None;

        state
            .save(&state_path)
            .context("failed to save default state")?;

        Ok(())
    }
}
