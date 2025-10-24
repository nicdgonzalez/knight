use std::thread;
use std::time::{Duration, Instant};

use anyhow::Context as _;
use chrono::{Local, NaiveTime};
use reqwest::blocking::Client;
use tracing::{error, warn};

use crate::cache::Cache;
use crate::commands::Run;
use crate::config::{Config, Location};
use crate::daylight::Daylight;
use crate::state::{State, get_current_theme};
use crate::theme::{Theme, update_system_theme};

#[derive(Debug, clap::Args)]
pub struct Start {
    #[clap(
        short,
        long,
        default_value = "3000",
        help = "Time to wait between checks (in milliseconds)"
    )]
    interval: u64,
}

impl Run for Start {
    fn run(&self) -> anyhow::Result<()> {
        let state_path = {
            let mut path = dirs::data_dir().context("failed to get data directory")?;
            path.extend(["knight", "state.json"]);
            path
        };
        let config_path = {
            let mut path =
                dirs::config_local_dir().context("failed to get user's config directory")?;
            path.extend(["knight", "Knight.toml"]);
            path
        };
        let cache_path = {
            let mut path = dirs::cache_dir().context("failed to get cache directory")?;
            path.extend(["knight", "cache.json"]);
            path
        };
        let interval = Duration::from_millis(self.interval);
        let client = Client::new();

        loop {
            let next_tick = Instant::now() + interval;

            let mut state = if let Ok(mut state) = State::from_file(&state_path) {
                let theme = get_current_theme().unwrap_or_else(|err| {
                    warn!("failed to get current theme: {err}");
                    Theme::Light
                });

                if state.theme != theme {
                    state.theme = theme;

                    if let Err(err) = state.save(&state_path) {
                        error!("failed to save default state: {err}");
                    }
                }

                state
            } else {
                let state = State::default();

                if let Err(err) = state.save(&state_path) {
                    error!("failed to save default state: {err}");
                }

                state
            };

            if state.disabled {
                let tick = Instant::now();

                if tick < next_tick {
                    thread::sleep(next_tick - tick);
                }

                continue;
            }

            let now = Local::now();

            let mut config = if let Ok(config) = Config::from_file(&config_path) {
                config
            } else {
                let config = Config::default();

                if let Err(err) = config.save(&config_path) {
                    error!("failed to save default config: {err}");
                }

                config
            };

            let mut cache = if let Ok(cache) = Cache::from_file(&cache_path) {
                cache
            } else {
                let cache = Cache::default();

                if let Err(err) = cache.save(&cache_path) {
                    error!("failed to save default cache: {err}");
                }

                cache
            };

            if config.location.is_enabled()
                && !config.location.is_defined()
                && cache
                    .location_last_attempt
                    .is_none_or(|d| d != now.date_naive())
            {
                if let Some((longitude, latitude)) = get_location(&client) {
                    config.location = Location {
                        longitude: Some(longitude),
                        latitude: Some(latitude),
                        ..config.location
                    };

                    if let Err(err) = config.save(&config_path) {
                        error!("failed to save updated config: {err}");
                    }
                }
            }

            let (sunrise, sunset) = if config.location.is_defined() {
                if cache.last_updated == now.date_naive()
                    && matches!(
                        (cache.daylight.sunrise, cache.daylight.sunset),
                        (Some(_), Some(_))
                    )
                {
                    (
                        cache.daylight.sunrise.unwrap(),
                        cache.daylight.sunset.unwrap(),
                    )
                } else {
                    let longitude = config.location.longitude.unwrap();
                    let latitude = config.location.latitude.unwrap();
                    let daylight = get_sunrise_sunset(longitude, latitude, &client)
                        .unwrap_or((config.fallback.sunrise, config.fallback.sunset));

                    cache = Cache {
                        last_updated: now.date_naive(),
                        daylight: Daylight {
                            sunrise: Some(daylight.0),
                            sunset: Some(daylight.1),
                        },
                        ..cache
                    };
                    _ = cache.save(&cache_path);

                    daylight
                }
            } else {
                (config.fallback.sunrise, config.fallback.sunset)
            };

            let is_day = if sunrise < sunset {
                now.time() >= sunrise && now.time() < sunset
            } else {
                now.time() >= sunrise || now.time() < sunset
            };

            if is_day {
                if state.theme != Theme::Light {
                    if let Err(err) = update_system_theme(Theme::Light) {
                        error!("failed to set light theme: {err}");
                    }

                    state.theme = Theme::Light;
                    if let Err(err) = state.save(&state_path) {
                        error!("failed to save updated state: {err}");
                    }
                }
            } else if state.theme != Theme::Dark {
                if let Err(err) = update_system_theme(Theme::Dark) {
                    error!("failed to set dark theme: {err}");
                }

                state.theme = Theme::Dark;
                if let Err(err) = state.save(&state_path) {
                    error!("failed to save updated state: {err}");
                }
            }

            let tick = Instant::now();
            if tick < next_tick {
                thread::sleep(next_tick - tick);
            }
        }
    }
}

fn get_sunrise_sunset(
    longitude: f32,
    latitude: f32,
    client: &Client,
) -> Option<(NaiveTime, NaiveTime)> {
    #[derive(serde::Deserialize)]
    struct Response {
        results: Results,
    }

    #[derive(serde::Deserialize)]
    struct Results {
        sunrise: NaiveTime,
        sunset: NaiveTime,
    }

    let url =
        format!("https://api.sunrisesunset.io/json?lng={longitude}&lat={latitude}&time_format=24");

    let response = client
        .get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .ok()?;
    let text = response.text().ok()?;
    let data = serde_json::from_str::<Response>(&text).ok()?;

    Some((data.results.sunrise, data.results.sunset))
}

fn get_location(client: &Client) -> Option<(f32, f32)> {
    #[derive(Debug, serde::Deserialize)]
    struct Response {
        longitude: f32,
        latitude: f32,
    }

    let response = client
        .get("https://freeipapi.com/api/json/")
        .timeout(Duration::from_secs(10))
        .send()
        .ok()?;
    let text = response.text().ok()?;
    let data = serde_json::from_str::<Response>(&text)
        .inspect_err(|err| error!("failed to parse response: {err}"))
        .ok()?;

    Some((data.longitude, data.latitude))
}
