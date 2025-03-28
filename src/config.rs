#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub struct Configuration {
    pub fallback: Fallback,
    pub location: Location,
}

/// Fallback for when location-based sunrise/sunset times are unavailable.
#[derive(Debug, serde::Deserialize)]
#[serde(default)]
pub struct Fallback {
    pub sunrise: chrono::NaiveTime,
    pub sunset: chrono::NaiveTime,
}

impl std::default::Default for Fallback {
    fn default() -> Self {
        Self {
            sunrise: chrono::NaiveTime::from_hms_opt(6, 30, 0).unwrap(),
            sunset: chrono::NaiveTime::from_hms_opt(18, 30, 0).unwrap(),
        }
    }
}

/// Location settings for getting sunrise/sunset times automatically.
#[derive(Debug, serde::Deserialize)]
#[serde(default)]
pub struct Location {
    pub enabled: bool,
    pub longitude: Option<f32>,
    pub latitude: Option<f32>,
}

impl std::default::Default for Location {
    fn default() -> Self {
        Self {
            enabled: true,
            longitude: None,
            latitude: None,
        }
    }
}
