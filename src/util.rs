use std::{collections::HashSet, path::PathBuf};

use device_query::{DeviceQuery, DeviceState, Keycode};

pub fn is_hotkey_pressed(device_state: &DeviceState) -> bool {
    HashSet::<Keycode>::from_iter(device_state.get_keys()).is_superset(&HashSet::from_iter([
        Keycode::LControl,
        Keycode::LAlt,
        Keycode::Backspace,
    ]))
}

#[cfg(target_os = "windows")]
pub fn get_shortcuts() -> Vec<PathBuf> {
    use std::path::Path;
    use walkdir::WalkDir;

    fn walkdir_to_vec(walkdir: WalkDir) -> Vec<PathBuf> {
        walkdir
            .into_iter()
            .filter_map(Result::ok)
            .filter(|x| x.path().extension().map(|e| e == "lnk").unwrap_or_default())
            .map(|x| x.path().to_owned())
            .collect()
    }

    Iterator::chain(
        walkdir_to_vec(WalkDir::new(
            Path::new(&std::env::var("AppData").unwrap())
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu"),
        ))
        .into_iter(),
        walkdir_to_vec(WalkDir::new(
            Path::new(&std::env::var("ProgramData").unwrap())
                .join("Microsoft")
                .join("Windows"),
        ))
        .into_iter(),
    )
    .collect()
}

#[cfg(target_os = "macos")]
pub fn get_shortcuts() -> Vec<PathBuf> {
    std::fs::read_dir("/Applications")
        .unwrap()
        .filter_map(Result::ok)
        .filter(|de| {
            let path = de.path();
            let file_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            !file_name.starts_with('.') && file_name.ends_with(".app")
        })
        .filter_map(|de| {
            Some(
                de.path()
                    .join("Contents")
                    .join("MacOS")
                    .join(de.path().file_name()?.to_str()?.strip_suffix(".app")?),
            )
        })
        .collect()
}
