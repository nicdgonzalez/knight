use std::thread;
use std::time::{Duration, Instant};

use anyhow::Context as _;
use chrono::{Days, Local, NaiveTime, TimeZone, Timelike};
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
        let mut state_path = dirs::data_dir().context("failed to get data directory")?;
        state_path.extend(["knight", "state.json"]);
        let state_path = state_path;

        let mut config_path =
            dirs::config_local_dir().context("failed to get user's config directory")?;
        config_path.extend(["knight", "Knight.toml"]);
        let config_path = config_path;

        let mut cache_path = dirs::cache_dir().context("failed to get cache directory")?;
        cache_path.extend(["knight", "cache.json"]);
        let cache_path = cache_path;

        let interval = Duration::from_millis(self.interval);
        let client = Client::new();

        loop {
            let next_tick = Instant::now() + interval;

            let current_theme = get_current_theme()
                .inspect_err(|err| warn!("failed to get current theme: {err}"))
                .unwrap_or(Theme::Light);

            let mut state = if let Ok(state) = State::from_file(&state_path) {
                state
            } else {
                let state = State::default();
                if let Err(err) = state.save(&state_path) {
                    error!("failed to save default state: {err}");
                }
                state
            };

            if state.disabled {
                maybe_sleep(next_tick);
                continue;
            }

            let now = Local::now();

            if let Some(until) = state.manually_override_until {
                if now < until {
                    maybe_sleep(next_tick);
                    continue;
                } else {
                    state.manually_override_until = None;
                }
            }

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
                    // To avoid spamming an external API, we only allow one attempt per day.
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
                if cache.last_updated == now.date_naive() && cache.daylight.is_defined() {
                    let Daylight { sunrise, sunset } = cache.daylight;
                    (sunrise.unwrap(), sunset.unwrap())
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

            if let Some(last_known_theme) = state.last_known_theme {
                if current_theme != last_known_theme && state.manually_override_until.is_none() {
                    warn!("user manually changed system theme to: {:?}", current_theme);

                    state.theme = current_theme;
                    state.last_known_theme = Some(current_theme);

                    let next_transition = if is_day {
                        NaiveTime::from_hms_opt(sunset.hour(), sunset.minute(), 0).unwrap()
                    } else {
                        NaiveTime::from_hms_opt(sunrise.hour(), sunrise.minute(), 0).unwrap()
                    };
                    let date = if is_day && sunset < sunrise {
                        now.date_naive().checked_add_days(Days::new(1)).unwrap()
                    } else {
                        now.date_naive()
                    };
                    let dt = date.and_time(next_transition);

                    state.manually_override_until = Some(Local.from_local_datetime(&dt).unwrap());

                    if let Err(err) = state.save(&state_path) {
                        error!("failed to save updated state: {err}");
                    }

                    maybe_sleep(next_tick);
                    continue;
                }
            } else {
                state.last_known_theme = Some(current_theme);
                if let Err(err) = state.save(&state_path) {
                    error!("failed to save updated state: {err}");
                }
            }

            if is_day {
                if state.theme != Theme::Light {
                    if let Err(err) = update_system_theme(Theme::Light) {
                        error!("failed to set light theme: {err}");
                    }

                    state.theme = Theme::Light;
                    state.last_known_theme = Some(state.theme);
                    if let Err(err) = state.save(&state_path) {
                        error!("failed to save updated state: {err}");
                    }
                }
            } else if state.theme != Theme::Dark {
                if let Err(err) = update_system_theme(Theme::Dark) {
                    error!("failed to set dark theme: {err}");
                }

                state.theme = Theme::Dark;
                state.last_known_theme = Some(state.theme);
                if let Err(err) = state.save(&state_path) {
                    error!("failed to save updated state: {err}");
                }
            }

            maybe_sleep(next_tick);
        }
    }
}

fn maybe_sleep(next_tick: Instant) {
    let tick = Instant::now();
    if tick < next_tick {
        thread::sleep(next_tick - tick);
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
