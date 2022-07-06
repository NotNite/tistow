use std::path::{Path, PathBuf};

use device_query::{DeviceQuery, DeviceState, Keycode};
use walkdir::WalkDir;

use crate::config::Config;

pub fn is_hotkey_pressed(device_state: &DeviceState) -> bool {
    let keys: Vec<Keycode> = device_state.get_keys();
    // TODO: turn into cli args
    let hotkey = vec![Keycode::LControl, Keycode::LAlt, Keycode::Backspace];

    let mut hotkey_pressed = true;

    for key in hotkey {
        if !keys.contains(&key) {
            hotkey_pressed = false;
        }
    }

    hotkey_pressed
}

pub fn get_shortcuts(config: &Config) -> Vec<PathBuf> {
    let mut result: Vec<PathBuf> = Vec::new();

    for path in &config.search.shortcut_paths {
        let dir = shellexpand::env(&path)
            .expect("couldn't get shortcut path")
            .to_string();
        let path = Path::new(&dir);
        let shortcuts_dir = WalkDir::new(path);

        println!("{:#?}", dir);

        let mut shortcuts: Vec<PathBuf> = shortcuts_dir
            .into_iter()
            .filter(|x| {
                if let Ok(x) = x {
                    let path = x.path().to_str().unwrap();
                    let lowercase = path.to_lowercase();

                    let mut ignored = false;
                    for ignore_str in &config.search.ignore_paths {
                        let ignore_dir = shellexpand::env(&ignore_str)
                            .expect("couldn't get shortcut ignore dir")
                            .to_string();

                        if lowercase.contains(&ignore_dir.to_lowercase()) {
                            ignored = true;
                        }
                    }

                    !ignored && lowercase.ends_with(".lnk")
                } else {
                    false
                }
            })
            .map(|x| x.unwrap().path().to_owned())
            .collect();

        result.append(&mut shortcuts);
    }

    println!("{:#?}", result);

    result
}
