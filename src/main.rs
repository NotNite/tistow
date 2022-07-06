use clap::Parser;
use device_query::DeviceState;
use eframe::egui;
use egui::{Pos2, Vec2};
use search::Search;

mod app;
mod search;
mod util;

use app::App;
use util::is_hotkey_pressed;

fn spawn_window(x: f32, y: f32, aggregator: Search) -> ! {
    let options = eframe::NativeOptions {
        transparent: true,
        resizable: false,
        always_on_top: true,
        decorated: false,
        initial_window_size: Some(Vec2 { x: 640., y: 320. }),
        initial_window_pos: Some(Pos2 { x, y }),
        ..eframe::NativeOptions::default()
    };

    eframe::run_native(
        "tistow",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());

            Box::new(App::new(aggregator))
        }),
    );
}

#[derive(Parser, Debug)]
struct Args {
    /// X coordinate of the window
    #[clap(default_value_t = 640.)]
    x: f32,
    /// Y coordinate of the window
    #[clap(default_value_t = 380.)]
    y: f32,
}

fn main() {
    let args = Args::parse();

    // spawn when hotkey first pressed
    let device_state = DeviceState::new();
    let aggregator = Search::new();

    loop {
        if is_hotkey_pressed(&device_state) {
            spawn_window(args.x, args.y, aggregator);
        }
    }
}
