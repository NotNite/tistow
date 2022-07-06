use device_query::DeviceState;
use eframe::egui;
use egui::{Pos2, Vec2};

mod app;
mod config;
mod search;
mod util;

use app::App;
use config::Config;
use util::is_hotkey_pressed;

fn spawn_window(config: Config) -> ! {
    let options = eframe::NativeOptions {
        transparent: true,
        resizable: false,
        always_on_top: true,
        decorated: false,
        initial_window_size: Some(Vec2 {
            x: config.window.width as f32,
            y: config.window.height as f32,
        }),
        initial_window_pos: Some(Pos2 {
            x: config.window.x as f32,
            y: config.window.y as f32,
        }),
        ..eframe::NativeOptions::default()
    };

    eframe::run_native(
        "tistow",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());

            Box::new(App::new(config))
        }),
    );
}

fn main() {
    let config = config::get_config();
    println!("{:#?}", config);

    // spawn when hotkey first pressed
    let device_state = DeviceState::new();

    loop {
        if is_hotkey_pressed(&device_state, &config.general.hotkey) {
            spawn_window(config);
        }
    }
}
