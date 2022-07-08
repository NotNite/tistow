use directories::ProjectDirs;
use figment::{
    providers::{Format, Serialized, Toml},
    value::Map,
    Figment,
};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Search {
    pub shortcut_paths: Vec<String>,
    pub ignore_paths: Vec<String>,
    pub aliases: Map<String, String>,
}

impl Default for Search {
    #[cfg(target_os = "windows")]
    fn default() -> Self {
        Self {
            shortcut_paths: vec![
                "${AppData}\\Microsoft\\Windows\\Start Menu".to_string(),
                "${ProgramData}\\Microsoft\\Windows\\Start Menu".to_string(),
            ],
            ignore_paths: vec![
                "${AppData}\\Microsoft\\Windows\\Start Menu\\Programs\\Startup".to_string(),
            ],
            aliases: Map::new(),
        }
    }

    #[cfg(target_os = "macos")]
    fn default() -> Self {
        Self {
            shortcut_paths: vec![
                "/Applications".to_string(),
                "/System/Applications".to_string(),
                "${HOME}/Applications".to_string(),
            ],
            ignore_paths: vec![],
            aliases: Map::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Config {
    pub window: Window,
    pub search: Search,
    pub general: General,
}

pub fn get_config() -> Config {
    // make dir
    let project_dir = ProjectDirs::from("", "", "tistow").expect("couldn't get project dir");
    let config_dir = project_dir.config_dir();

    fs::create_dir_all(config_dir).expect("couldn't create config dir");

    // create file if it doesn't exist
    let config_path = config_dir.join("config.toml");
    if !config_path.exists() {
        File::create(&config_path).expect("couldn't create config");
    }

    // read config
    let config: Config = Figment::from(Serialized::defaults(Config::default()))
        .merge(Toml::file(&config_path))
        .extract()
        .expect("couldn't load config");

    // write config
    fs::write(
        &config_path,
        toml::to_string(&config).expect("couldn't save config"),
    )
    .expect("couldn't save config");

    config
}
