use std::fs;
use std::path::Path;

use anyhow::Context as _;
use chrono::NaiveDate;

use crate::daylight::Daylight;
use crate::persistent::Persistent;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Cache {
    pub last_updated: NaiveDate,

    #[serde(flatten)]
    pub daylight: Daylight,

    pub location_last_attempt: Option<NaiveDate>,
}

impl Persistent for Cache {
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
