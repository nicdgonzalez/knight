use std::cmp::Ordering;
use std::str::FromStr;

use crate::commands;
use crate::config;

pub fn run() -> Result<(), commands::Error> {
    let lock_file = crate::get_lock_file();
    std::fs::create_dir_all(lock_file.parent().unwrap())?;
    let now = chrono::Local::now();

    // If the file is empty, it means it is disabled indefinitely. Otherwise,
    // it should contain the last date for which the application is disabled.
    if let Ok(content) = std::fs::read_to_string(&lock_file) {
        if &content == "" {
            return Ok(());
        }

        match chrono::NaiveDate::from_str(&content) {
            Ok(date) => match date.cmp(&now.date_naive()) {
                // The application is currently disabled.
                Ordering::Equal | Ordering::Greater => return Ok(()),
                Ordering::Less => {
                    // The date already passed. Remove lock.
                    std::fs::remove_file(&lock_file)?;
                }
            },
            Err(_) => {
                // The lock's contents are invalid. Remove lock.
                std::fs::remove_file(&lock_file)?;
            }
        };
    }

    let config: config::Configuration = {
        let file = crate::get_config_home().join("Knight.toml");
        std::fs::create_dir_all(file.parent().unwrap())?;

        match std::fs::read_to_string(&file) {
            Ok(content) => toml::from_str(&content)?,
            Err(err) => match err.kind() {
                std::io::ErrorKind::NotFound => toml::from_str(include_str!("../../Knight.toml"))?,
                _ => return Err(err.into()),
            },
        }
    };

    let (sunrise, sunset) = if config.location.enabled {
        let client = reqwest::blocking::Client::new();
        let location = get_location(&client, &config.location);
        get_times(&client, &location, &config.fallback)
    } else {
        (config.fallback.sunrise, config.fallback.sunset)
    };

    if now.time() >= sunrise && now.time() < sunset {
        crate::set_light_theme()?;
    } else {
        crate::set_dark_theme()?;
    }

    Ok(())
}

#[derive(serde::Deserialize)]
struct LocationResponse {
    longitude: f32,
    latitude: f32,
}
// TODO: Cache response once a day.

fn get_location(
    client: &reqwest::blocking::Client,
    location: &config::Location,
) -> config::Location {
    if let (None, None) = (location.longitude, location.latitude) {
        let url = "https://freeipapi.com/api/json/";

        match client.get(url).send() {
            Ok(response) => {
                let body = response.text().expect("expected response to be valid json");
                let data: LocationResponse = serde_json::from_str(&body).unwrap();

                return config::Location {
                    enabled: location.enabled,
                    longitude: Some(data.longitude),
                    latitude: Some(data.latitude),
                };
            }
            Err(_) => {
                return config::Location {
                    enabled: true,
                    longitude: None,
                    latitude: None,
                };
            }
        };
    };

    config::Location {
        enabled: location.enabled,
        longitude: location.longitude,
        latitude: location.latitude,
    }
}

#[derive(serde::Deserialize)]
struct TimeResponse {
    results: Results,
}

#[derive(serde::Deserialize)]
struct Results {
    sunrise: chrono::NaiveTime,
    sunset: chrono::NaiveTime,
}

// TODO: Cache response once a day.

fn get_times(
    client: &reqwest::blocking::Client,
    location: &config::Location,
    fallback: &config::Fallback,
) -> (chrono::NaiveTime, chrono::NaiveTime) {
    match (location.longitude, location.latitude) {
        (Some(longitude), Some(latitude)) => {
            let url = format!(
                "https://api.sunrisesunset.io/json?lng={longitude}&lat={latitude}&time_format=24",
            );

            match client.get(&url).send() {
                Ok(response) => {
                    let body = response.text().expect("expected response to be valid json");
                    let data: TimeResponse = serde_json::from_str(&body).unwrap();
                    (data.results.sunrise, data.results.sunset)
                }
                Err(_) => return (fallback.sunrise, fallback.sunset),
            }
        }
        (Some(_), None) | (None, Some(_)) => {
            panic!("both longitude and latitude must be set")
        }
        (None, None) => {
            // This case happens when the `location` feature is enabled,
            // but there was a problem getting the user's location.
            (fallback.sunrise, fallback.sunset)
        }
    }
}
