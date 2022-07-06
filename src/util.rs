use std::path::{Path, PathBuf};

use device_query::{DeviceQuery, DeviceState, Keycode};
use walkdir::WalkDir;

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

fn walkdir_to_vec(walkdir: WalkDir) -> Vec<PathBuf> {
    let deez: Vec<PathBuf> = walkdir
        .into_iter()
        .filter(|x| {
            if let Ok(x) = x {
                let path = x.path().to_str().unwrap();

                path.to_lowercase().ends_with(".lnk")
            } else {
                false
            }
        })
        .map(|x| x.unwrap().path().to_owned())
        .collect();

    deez
}

pub fn get_shortcuts() -> Vec<PathBuf> {
    let mut result: Vec<PathBuf> = Vec::new();

    let appdata = Path::new(&std::env::var("AppData").unwrap())
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu");
    let programdata = Path::new(&std::env::var("ProgramData").unwrap())
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu");

    let mut appdata_options: Vec<PathBuf> = walkdir_to_vec(WalkDir::new(appdata));
    let mut programdata_options: Vec<PathBuf> = walkdir_to_vec(WalkDir::new(programdata));

    result.append(&mut appdata_options);
    result.append(&mut programdata_options);

    result
}
