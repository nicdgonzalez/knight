use crate::commands;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum Theme {
    Light,
    Dark,
}

#[derive(Debug, clap::Args)]
pub struct Args {
    #[arg(value_enum)]
    pub theme: Theme,
}

pub fn run(args: &Args) -> Result<(), commands::Error> {
    let config_home = crate::get_config_home();
    std::fs::create_dir_all(&config_home)?;
    let disabled = config_home.join(".disabled");
    let today = {
        let now = chrono::Local::now();
        now.to_rfc3339().split_once("T").unwrap().0.to_owned()
    };
    std::fs::write(disabled, today)?;

    match args.theme {
        Theme::Light => {
            crate::set_light_theme()?;
            eprintln!("Successfully switched to light theme.");
        }
        Theme::Dark => {
            crate::set_dark_theme()?;
            eprintln!("Successfully switched to dark theme.");
        }
    };

    Ok(())
}
