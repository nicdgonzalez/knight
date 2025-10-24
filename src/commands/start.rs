use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Context as _;
use chrono::{DateTime, Days, Local, NaiveDate, NaiveTime, TimeZone, Timelike};
use reqwest::blocking::Client;
use tracing::{error, warn};

use crate::cache::Cache;
use crate::commands::Run;
use crate::config::{Config, Fallback, Location};
use crate::daylight::Daylight;
use crate::persistent::{Persistent, load_or_default};
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

        let mut config_path = dirs::config_local_dir().context("failed to get config directory")?;
        config_path.extend(["knight", "Knight.toml"]);
        let config_path = config_path;

        let mut cache_path = dirs::cache_dir().context("failed to get cache directory")?;
        cache_path.extend(["knight", "cache.json"]);
        let cache_path = cache_path;

        let interval = Duration::from_millis(self.interval);

        loop {
            let next_tick = Instant::now() + interval;
            let now = Local::now();

            let mut state = load_or_default::<State>(&state_path);

            if is_disabled(&now, &mut state) {
                maybe_sleep(next_tick);
                continue;
            }

            let (sunrise, sunset) = get_daylight(&now, &config_path, &cache_path);

            let current_theme = get_current_theme()
                .inspect_err(|err| warn!("failed to get current theme: {err}"))
                .unwrap_or(Theme::Light);

            if state.theme != current_theme {
                handle_theme_changed_manually(
                    current_theme,
                    &now,
                    sunrise,
                    sunset,
                    &mut state,
                    &state_path,
                );
                maybe_sleep(next_tick);
                continue;
            }

            let target_theme = if is_daytime(&now, sunrise, sunset) {
                Theme::Light
            } else {
                Theme::Dark
            };

            apply_theme(target_theme, &mut state, &state_path);
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

/// Checks whether the program is currently disabled.
///
/// The program is disabled if:
///
/// - User explicitly disabled it using the `disable` command.
/// - User manually changed the system theme.
fn is_disabled(now: &DateTime<Local>, state: &mut State) -> bool {
    state.disabled || state.override_until.take_if(|until| now < until).is_some()
}

/// Gets today's sunrise and sunset times.
fn get_daylight(
    now: &DateTime<Local>,
    config_path: &Path,
    cache_path: &Path,
) -> (NaiveTime, NaiveTime) {
    let mut config = load_or_default::<Config>(config_path);
    let mut cache = load_or_default::<Cache>(cache_path);
    let client = Client::new();

    if is_ready_for_get_location_attempt(now, &config.location, cache.location_last_attempt) {
        get_location(&client, &mut config, config_path);
    }

    let (sunrise, sunset) = get_daylight_inner(
        &client,
        now,
        &config.location,
        &config.fallback,
        &mut cache,
        cache_path,
    );

    (sunrise, sunset)
}

/// Checks if we need and are allowed to call the external API to get the user's location.
///
/// This function returns true if:
///
/// - Location feature was not explicitly turned off in the configuration file.
/// - Location has not been defined yet.
/// - We have not already tried to get the location today.
fn is_ready_for_get_location_attempt(
    now: &DateTime<Local>,
    location: &Location,
    last_attempt: Option<NaiveDate>,
) -> bool {
    location.is_enabled()
        && !location.is_defined()
        && last_attempt.is_none_or(|d| d != now.date_naive())
}

/// Get the user's location and save it to the configuration file.
fn get_location(client: &Client, config: &mut Config, config_path: &Path) {
    if let Some((longitude, latitude)) = get_location_inner(client) {
        config.location = Location {
            longitude: Some(longitude),
            latitude: Some(latitude),
            ..config.location
        };

        if let Err(err) = config.save(config_path) {
            error!("failed to save updated config: {err}");
        }
    }
}

/// Calls an external API to get the user's current location.
fn get_location_inner(client: &Client) -> Option<(f32, f32)> {
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

fn get_daylight_inner(
    client: &Client,
    now: &DateTime<Local>,
    location: &Location,
    fallback: &Fallback,
    cache: &mut Cache,
    cache_path: &Path,
) -> (NaiveTime, NaiveTime) {
    if location.is_defined() {
        if cache.last_updated == now.date_naive() && cache.daylight.is_defined() {
            let Daylight { sunrise, sunset } = cache.daylight;
            (sunrise.unwrap(), sunset.unwrap())
        } else {
            let longitude = location.longitude.unwrap();
            let latitude = location.latitude.unwrap();
            let daylight = get_daylight_inner_inner(longitude, latitude, client)
                .unwrap_or((fallback.sunrise, fallback.sunset));

            *cache = Cache {
                last_updated: now.date_naive(),
                daylight: Daylight {
                    sunrise: Some(daylight.0),
                    sunset: Some(daylight.1),
                },
                ..*cache
            };

            if let Err(err) = cache.save(cache_path) {
                error!("failed to update cache: {err}");
            }

            daylight
        }
    } else {
        (fallback.sunrise, fallback.sunset)
    }
}

// (In the market for a better name...)
fn get_daylight_inner_inner(
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

/// Checks whether `now` falls between sunrise and sunset.
///
/// This function also handles if sunset rolls into the following day.
fn is_daytime(now: &DateTime<Local>, sunrise: NaiveTime, sunset: NaiveTime) -> bool {
    if sunrise < sunset {
        now.time() >= sunrise && now.time() < sunset
    } else {
        now.time() >= sunrise || now.time() < sunset
    }
}

fn handle_theme_changed_manually(
    current_theme: Theme,
    now: &DateTime<Local>,
    sunrise: NaiveTime,
    sunset: NaiveTime,
    state: &mut State,
    state_path: &Path,
) {
    warn!("user manually changed system theme to: {:?}", current_theme);

    state.theme = current_theme;
    state.override_until = Some(get_next_cycle(now, sunset, sunrise));

    if let Err(err) = state.save(state_path) {
        error!("failed to save updated state: {err}");
    }
}

/// When the user changes the theme manually, we temporarily disable the program until the
/// next-next time we're ready for a theme change. This is so we aren't fighting with the user
/// over the current theme, but we also don't want the user to forget to turn us back on.
fn get_next_cycle(now: &DateTime<Local>, sunset: NaiveTime, sunrise: NaiveTime) -> DateTime<Local> {
    let is_day = is_daytime(now, sunrise, sunset);

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

    let datetime = date.and_time(next_transition);
    Local.from_local_datetime(&datetime).unwrap()
}

/// Send the request to the system to update the user's theme.
fn apply_theme(target_theme: Theme, state: &mut State, state_path: &Path) {
    if state.theme != target_theme {
        if let Err(err) = update_system_theme(target_theme) {
            error!("failed to set light theme: {err}");
        }

        state.theme = target_theme;

        if let Err(err) = state.save(state_path) {
            error!("failed to save updated state: {err}");
        }
    }
}
