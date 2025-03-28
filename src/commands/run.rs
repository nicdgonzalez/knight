use std::str::FromStr;

use crate::commands;
use crate::config::Configuration;

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
                std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => {
                    return Ok(());
                }
                std::cmp::Ordering::Less => {
                    // The date already passed.
                    std::fs::remove_file(&lock_file)?;
                }
            },
            Err(_) => {
                // The file's contents are invalid.
                std::fs::remove_file(&lock_file)?;
            }
        };
    }

    let config: Configuration = {
        let file = crate::get_config_home().join("Knight.toml");

        match std::fs::read_to_string(&file) {
            Ok(content) => toml::from_str(&content)?,
            Err(err) => match err.kind() {
                std::io::ErrorKind::NotFound => {
                    // Load default configuration
                    toml::from_str(include_str!("../../Knight.toml"))?
                }
                _ => return Err(err.into()),
            },
        }
    };

    let (sunrise, sunset) = get_times(&config);

    if now.time() >= sunrise && now.time() < sunset {
        crate::set_light_theme()?;
    } else {
        crate::set_dark_theme()?;
    }

    Ok(())
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

fn get_times(config: &Configuration) -> (chrono::NaiveTime, chrono::NaiveTime) {
    if config.location.enabled {
        let client = reqwest::blocking::Client::new();
        let location = get_location(&config, &client);

        match location {
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
                    Err(_) => return (config.fallback.sunrise, config.fallback.sunset),
                }
            }
            (Some(_), None) | (None, Some(_)) => {
                panic!("both longitude and latitude must be set")
            }
            (None, None) => todo!(),
        };
    }

    (config.fallback.sunrise, config.fallback.sunset)
}

#[derive(serde::Deserialize)]
struct LocationResponse {
    longitude: f32,
    latitude: f32,
}

fn get_location(
    config: &Configuration,
    client: &reqwest::blocking::Client,
) -> (Option<f32>, Option<f32>) {
    if let (None, None) = (config.location.longitude, config.location.latitude) {
        let url = "https://freeipapi.com/api/json/";

        match client.get(url).send() {
            Ok(response) => {
                let body = response.text().expect("expected response to be valid json");
                let data: LocationResponse = serde_json::from_str(&body).unwrap();

                return (Some(data.longitude), Some(data.latitude));
            }
            Err(_) => return (None, None),
        };
    };

    (config.location.longitude, config.location.latitude)
}
