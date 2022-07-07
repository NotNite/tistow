use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::config::Config;
use device_query::{DeviceQuery, DeviceState, Keycode};

pub fn is_hotkey_pressed(device_state: &DeviceState, hotkey_str: &[Keycode]) -> bool {
    HashSet::<Keycode>::from_iter(device_state.get_keys())
        .is_superset(&HashSet::from_iter(hotkey_str.iter().copied()))
}

#[cfg(target_os = "windows")]
pub fn get_shortcuts(config: &Config) -> Vec<PathBuf> {
    config
        .search
        .shortcut_paths
        .iter()
        .map(|path| {
            walkdir::WalkDir::new(Path::new(
                shellexpand::env(&path)
                    .expect("couldn't get shortcut path")
                    .as_ref(),
            ))
        })
        .flat_map(|shortcuts_dir| {
            shortcuts_dir
                .into_iter()
                .filter_map(Result::ok)
                .filter(|x| {
                    let path = x.path().to_str().unwrap();
                    let lowercase = path.to_lowercase();
                    let ignored = config
                        .search
                        .ignore_paths
                        .iter()
                        .map(|ignore_str| {
                            shellexpand::env(&ignore_str)
                                .expect("couldn't get shortcut ignore dir")
                                .to_lowercase()
                        })
                        .any(|ignore_dir| lowercase.contains(&ignore_dir));

                    !ignored && (lowercase.ends_with(".lnk") || lowercase.ends_with(".url"))
                })
                .map(|x| x.path().to_owned())
        })
        .collect()
}

#[cfg(target_os = "macos")]
pub fn get_shortcuts(config: &Config) -> Vec<PathBuf> {
    config
        .search
        .shortcut_paths
        .iter()
        .map(|path| {
            walkdir::WalkDir::new(Path::new(
                shellexpand::env(&path)
                    .expect("couldn't get shortcut path")
                    .as_ref(),
            ))
            // arbitrary limit to prevent long load times with apps that store stuff incorrectly
            // (looking at you unity)
            .max_depth(5)
        })
        .flat_map(|shortcuts_dir| {
            shortcuts_dir
                .into_iter()
                .filter_map(Result::ok)
                .filter(|de| {
                    let path = de.path();
                    let file_name = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or_default();

                    !file_name.starts_with('.') && file_name.ends_with(".app")
                })
                .map(|de| de.path().to_path_buf())
        })
        .collect()
}
