use std::cmp::Ordering;
use std::str::FromStr;

use crate::commands;
use crate::config;
use crate::get_cache_home;

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

    let sunrise: chrono::NaiveTime;
    let sunset: chrono::NaiveTime;

    if config.location.enabled {
        let cache_home = get_cache_home();

        let time_file = cache_home.join("times").join(now.date_naive().to_string());
        if let Ok(times) = std::fs::read_to_string(&time_file) {
            // Get cached times.
            // TODO: Properly handle errors.
            let times = times.split_once(",").unwrap();
            sunrise = chrono::NaiveTime::from_str(times.0).unwrap();
            sunset = chrono::NaiveTime::from_str(times.1).unwrap();
        } else {
            let client = reqwest::blocking::Client::new();
            let location_file = cache_home.join("location.txt");

            let location: config::Location = {
                if let Ok(coordinates) = std::fs::read_to_string(&location_file) {
                    // Get cached location.
                    // TODO: Properly handle errors.
                    let coordinates = coordinates.split_once(",").unwrap();
                    config::Location {
                        enabled: config.location.enabled,
                        longitude: Some(coordinates.0.parse().unwrap()),
                        latitude: Some(coordinates.1.parse().unwrap()),
                    }
                } else {
                    get_location(&client, &config.location)
                }
            };

            (sunrise, sunset) = get_times(&client, &location, &config.fallback);
        };
    } else {
        (sunrise, sunset) = (config.fallback.sunrise, config.fallback.sunset);
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

// TODO: Add section to README regarding how to delete the cache file.

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

                // TODO: Don't panic on cache-related operations.
                let cache_file = get_cache_home().join("location.txt");
                std::fs::create_dir_all(&cache_file.parent().unwrap())
                    .expect("failed to create cache subdirectory");
                std::fs::write(&cache_file, format!("{},{}", data.longitude, data.latitude))
                    .expect("failed to cache location data");

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
    date: chrono::NaiveDate,
    sunrise: chrono::NaiveTime,
    sunset: chrono::NaiveTime,
}

// TODO: Add section to README regarding how to delete the cache file.

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

                    // TODO: Don't panic on cache-related operations.
                    let times_cache = get_cache_home().join("times");
                    std::fs::create_dir_all(&times_cache).unwrap();
                    let cache_file = times_cache.join(&data.results.date.to_string());
                    std::fs::write(
                        &cache_file,
                        format!("{},{}", data.results.sunrise, data.results.sunset),
                    )
                    .expect("failed to cache times data");

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
