use std::process::Command;

use anyhow::{Context as _, anyhow};

pub fn update_system_theme(theme: Theme) -> anyhow::Result<()> {
    let status = Command::new("gsettings")
        .args([
            "set",
            "org.gnome.desktop.interface",
            "color-scheme",
            theme.as_color_scheme(),
        ])
        .status()
        .context("failed to execute command")?;

    match status.code() {
        Some(0) => Ok(()),
        Some(code) => Err(anyhow!("failed with exit code: {code}")),
        None => Err(anyhow!("process terminated due to signal")),
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

impl Theme {
    pub const fn as_color_scheme(self) -> &'static str {
        match self {
            Self::Light => "default",
            Self::Dark => "prefer-dark",
        }
    }
}
