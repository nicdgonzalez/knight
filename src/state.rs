use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::Context as _;
use chrono::{DateTime, Local};
use tracing::{debug, warn};

use crate::{persistent::Persistent, theme::Theme};

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct State {
    pub theme: Theme,
    pub disabled: bool,
    pub override_until: Option<DateTime<Local>>,
}

impl Default for State {
    fn default() -> Self {
        let theme = get_current_theme().unwrap_or_else(|err| {
            warn!("failed to get current theme: {err}");
            Theme::Light
        });

        Self {
            theme,
            disabled: false,
            override_until: None,
        }
    }
}

impl Persistent for State {
    fn from_file(file: &Path) -> anyhow::Result<Self> {
        let text = fs::read_to_string(file).context("failed to read JSON file")?;
        let data = serde_json::from_str::<Self>(&text).context("failed to parse JSON contents")?;
        Ok(data)
    }

    fn save(&self, file: &Path) -> anyhow::Result<()> {
        let parent = file.parent().context("expected path to a file")?;
        fs::create_dir_all(parent)?;
        let contents = serde_json::to_string(self)?;
        fs::write(file, &contents)?;
        Ok(())
    }
}

pub fn get_current_theme() -> anyhow::Result<Theme> {
    let output = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
        .context("failed to run gsettings")?;

    let stdout = String::from_utf8(output.stdout).context("invalid UTF-8 in gsettings output")?;
    let value = stdout.trim().trim_matches('\'').to_lowercase();

    Ok(match value.as_str() {
        "default" => Theme::Light,
        "prefer-dark" => Theme::Dark,
        other => {
            debug!("unknown gsettings color-scheme value: {other}");
            Theme::Light
        }
    })
}
