use device_query::{DeviceQuery, DeviceState, Keycode};

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
