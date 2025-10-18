use std::fs;
use std::path::Path;

use anyhow::Context;
use chrono::NaiveTime;

/// Represents the `Knight.toml` configuration file.
#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Config {
    /// Default values for when location-based sunrise/sunset times are not available.
    pub fallback: Fallback,

    /// Geographical position for determining sunrise/sunset times.
    pub location: Location,
}

impl Config {
    pub fn from_file(file: &Path) -> anyhow::Result<Self> {
        let text = fs::read_to_string(file).context("failed to read TOML file")?;
        let data = toml::from_str::<Self>(&text).context("failed to parse TOML contents")?;
        Ok(data)
    }

    /// Write `self` to a file in `TOML` format.
    pub fn save(&self, file: &Path) -> anyhow::Result<()> {
        let parent = file.parent().context("expected path to a file")?;
        fs::create_dir_all(parent)?;
        let contents = toml::to_string(self)?;
        fs::write(file, &contents)?;
        Ok(())
    }
}

/// Represents default values for when location-based sunrise/sunset times are not available.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Fallback {
    /// Indicates the time to turn on Light mode.
    pub sunrise: NaiveTime,

    /// Indicates the time to turn on Dark mode.
    pub sunset: NaiveTime,
}

impl Default for Fallback {
    fn default() -> Self {
        Self {
            sunrise: NaiveTime::from_hms_opt(6, 30, 0)
                .expect("expected hardcoded sunrise to be valid"),
            sunset: NaiveTime::from_hms_opt(18, 30, 0)
                .expect("expected hardcoded sunset to be valid"),
        }
    }
}

/// Represents the user's geographical position for determining sunrise/sunset times.
#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct Location {
    /// Whether to attempt to use the user's location to determine sunrise/sunset times.
    pub disable: bool,

    /// Position on the Earth horizontally.
    pub longitude: Option<f32>,

    /// Position on the Earth vertically.
    pub latitude: Option<f32>,
}

impl Location {
    pub const fn is_enabled(&self) -> bool {
        !self.disable
    }

    pub const fn is_defined(&self) -> bool {
        matches!((self.longitude, self.latitude), (Some(_), Some(_)))
    }
}
