use std::{cmp::Ordering, str::FromStr};

#[derive(Debug, serde::Deserialize)]
struct Daylight {
    sunrise: chrono::NaiveTime,
    sunset: chrono::NaiveTime,
}

#[derive(Debug, serde::Deserialize)]
struct Geolocation {
    latitude: f32,
    longitude: f32,
}

/// Sets the system's theme based on the time of day.
pub(crate) fn run() -> Result<(), super::Error> {
    let now = chrono::Local::now();
    let today = now.date_naive();

    // Check if the user has disabled the application.
    if is_disabled(&today)? {
        return Ok(());
    }

    // Read the configuration file.
    let config = get_config()?;

    // Get sunrise/sunset times.
    let daylight = get_daylight(&today, &config.location, &config.fallback)?;

    // Set theme based on current time.
    if now.time() >= daylight.sunrise && now.time() < daylight.sunset {
        crate::set_light_theme()
    } else {
        crate::set_dark_theme()
    }
}

/// Determines if the application is disabled based on the presence and
/// contents of the `.disabled` file in `$XDG_CONFIG_HOME/knight`.
///
/// The application is considered disabled if:
///
/// - The file exists, and
/// - The file is either empty or contains a date that not yet passed.
///
/// An empty file indicates that the application is disabled indefinitely.
///
/// If the file contains a date in `YYYY-MM-DD` format, it specifies when the
/// file should be deleted, thereby re-enabling the application.
///
/// # Errors
///
/// This function returns an error if:
///
/// - The `.disabled` file cannot be read.
/// - The `.disabled` file cannot be removed.
fn is_disabled(today: &chrono::NaiveDate) -> Result<bool, super::Error> {
    let disabled_file = crate::get_disabled_file();

    if let Some(parent) = disabled_file.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if !disabled_file.exists() {
        return Ok(false);
    }

    // This file should contain the date for when to delete it.
    let content = std::fs::read_to_string(&disabled_file)?;

    if content.is_empty() {
        // An empty file indicates to disable the program indefinitely.
        return Ok(true);
    }

    match chrono::NaiveDate::from_str(&content) {
        Ok(date) => match date.cmp(&today) {
            Ordering::Equal | Ordering::Greater => Ok(true),
            Ordering::Less => {
                // The date has passed; re-enable the application.
                std::fs::remove_file(&disabled_file)?;
                Ok(false)
            }
        },
        Err(err) => {
            log::error!("request to disable the application denied: {err}");
            std::fs::remove_file(&disabled_file)?;
            Ok(false)
        }
    }
}

fn get_config() -> Result<crate::Config, super::Error> {
    let configuration_file = crate::get_config_home().join("Knight.toml");

    if let Some(parent) = configuration_file.parent() {
        std::fs::create_dir_all(parent)?;
    }

    match std::fs::read_to_string(&configuration_file) {
        Ok(content) => {
            let config: crate::Config = toml::from_str(&content)?;
            Ok(config)
        }
        Err(err) => match err.kind() {
            std::io::ErrorKind::NotFound => Ok(crate::Config::default()),
            _ => Err(err.into()),
        },
    }
}

fn get_daylight(
    today: &chrono::NaiveDate,
    location: &crate::Location,
    fallback: &crate::Fallback,
) -> Result<Daylight, super::Error> {
    if !location.enabled {
        // Dynamic sunrise/sunset times were disabled.
        return Ok(Daylight {
            sunrise: fallback.sunrise,
            sunset: fallback.sunset,
        });
    }

    // If the application ran already today, the sunrise/sunset times should
    // have been cached to reduce load on external APIs.
    let cache_file = crate::get_cache_home()
        .join("times")
        .join(today.to_string());

    if let Ok(content) = std::fs::read_to_string(&cache_file) {
        if let Some(daylight) = parse_cached_times(&content) {
            return Ok(daylight);
        }

        // Failed to get valid times from cached values. Fallthrough.
    }

    // Call external API to get sunrise/sunset times.
    let client = reqwest::blocking::Client::new();
    let geolocation = match (location.latitude, location.longitude) {
        (Some(latitude), Some(longitude)) => Some(Geolocation {
            latitude,
            longitude,
        }),
        _ => get_geolocation(&client)?,
    };

    if let Some(g) = geolocation {
        #[derive(Debug, serde::Deserialize)]
        struct Response {
            results: Daylight,
        }

        let latitude = g.latitude;
        let longitude = g.longitude;

        let url = format!(
            "https://api.sunrisesunset.io/json\
            ?lat={latitude}\
            &lng={longitude}\
            &time_format=24"
        );

        if let Ok(response) = client.get(url).send() {
            let body = response.text().expect("failed to decode response");
            let data: Response =
                serde_json::from_str(&body).expect("expected body to be valid json");

            let contents = format!("{},{}", data.results.sunrise, data.results.sunset);
            std::fs::write(&cache_file, &contents)?;

            return Ok(data.results);
        }
    }

    Ok(Daylight {
        sunrise: fallback.sunrise,
        sunset: fallback.sunset,
    })
}

fn parse_cached_times(content: &str) -> Option<Daylight> {
    if let Some((sr, ss)) = content.split_once(",") {
        if let (Ok(sunrise), Ok(sunset)) = (
            chrono::NaiveTime::from_str(sr),
            chrono::NaiveTime::from_str(ss),
        ) {
            return Some(Daylight { sunrise, sunset });
        }
    }

    None
}

fn get_geolocation(
    client: &reqwest::blocking::Client,
) -> Result<Option<Geolocation>, super::Error> {
    let cache_file = crate::get_cache_home().join("location.txt");

    if let Some(parent) = cache_file.parent() {
        std::fs::create_dir_all(&parent)?;
    }

    if let Ok(content) = std::fs::read_to_string(&cache_file) {
        if let Some(geolocation) = parse_cached_geolocation(&content) {
            return Ok(Some(geolocation));
        }
    }

    if let Ok(response) = client.get("https://freeipapi.com/api/json/").send() {
        let body = response.text().expect("failed to decode response");
        let data: Geolocation =
            serde_json::from_str(&body).expect("expected body to be valid json");

        let cache_contents = format!("{},{}", data.latitude, data.longitude);
        std::fs::write(&cache_file, &cache_contents)?;

        return Ok(Some(data));
    }

    Ok(None)
}

fn parse_cached_geolocation(content: &str) -> Option<Geolocation> {
    if let Some((lat, lng)) = content.split_once(",") {
        if let (Ok(latitude), Ok(longitude)) = (lat.parse::<f32>(), lng.parse::<f32>()) {
            return Some(Geolocation {
                latitude,
                longitude,
            });
        }
    }

    None
}
