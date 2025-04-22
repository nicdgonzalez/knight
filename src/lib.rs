#![warn(missing_docs)]

pub mod commands;
mod config;

pub(crate) use config::{Config, Fallback, Location};

pub fn get_config_home() -> std::path::PathBuf {
    dirs::config_dir()
        .expect("unable to get user's config directory")
        .join("knight")
}

pub fn get_cache_home() -> std::path::PathBuf {
    dirs::cache_dir()
        .expect("failed to get user's cache directory")
        .join("knight")
}

pub fn get_disabled_file() -> std::path::PathBuf {
    get_config_home().join(".disabled")
}

pub fn set_light_theme() -> Result<(), commands::Error> {
    std::process::Command::new("gsettings")
        .args([
            "set",
            "org.gnome.desktop.interface",
            "color-scheme",
            "default",
        ])
        .status()?;

    Ok(())
}

pub fn set_dark_theme() -> Result<(), commands::Error> {
    std::process::Command::new("gsettings")
        .args([
            "set",
            "org.gnome.desktop.interface",
            "color-scheme",
            "prefer-dark",
        ])
        .status()?;

    Ok(())
}
