use figment::{
    providers::{Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    path::Path,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Window {
    pub width: u32,
    pub height: u32,
    pub x: u32,
    pub y: u32,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            width: 640,
            height: 320,
            x: 640,
            y: 380,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Search {
    pub shortcut_paths: Vec<String>,
    pub ignore_paths: Vec<String>,
}

impl Default for Search {
    fn default() -> Self {
        Self {
            shortcut_paths: vec![
                "${AppData}\\Microsoft\\Windows\\Start Menu".to_string(),
                "${ProgramData}\\Microsoft\\Windows\\Start Menu".to_string(),
            ],
            ignore_paths: vec![
                "${AppData}\\Microsoft\\Windows\\Start Menu\\Programs\\Startup".to_string(),
            ],
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct General {
    pub hotkey: Vec<String>,
}

impl Default for General {
    fn default() -> Self {
        Self {
            hotkey: vec!["LAlt".to_string(), "Backspace".to_string()],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    pub window: Window,
    pub search: Search,
    pub general: General,
}

pub fn get_config() -> Config {
    // make dir
    let appdata_env = std::env::var("AppData").expect("couldn't get AppData");
    let appdata = Path::new(&appdata_env);
    fs::create_dir_all(appdata.join("tistow")).expect("couldn't create config dir");

    // create file if it doesn't exist
    let path = appdata.join("tistow").join("config.toml");
    if !path.exists() {
        File::create(&path).expect("couldn't create config");
    }

    // read config
    let config: Config = Figment::from(Serialized::defaults(Config::default()))
        .merge(Toml::file(&path))
        .extract()
        .expect("couldn't load config");

    // write config
    fs::write(
        &path,
        toml::to_string(&config).expect("couldn't save config"),
    )
    .expect("couldn't save config");

    config
}
