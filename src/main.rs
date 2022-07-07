use clap::Parser;
use eframe::egui;
use egui::{Pos2, Vec2};
use search::Search;

mod app;
mod search;
mod util;

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
    let aggregator = Search::new();

    let Args { x, y, .. } = args;
    eframe::run_native(
        "tistow",
        eframe::NativeOptions {
            transparent: true,
            resizable: false,
            always_on_top: true,
            decorated: false,
            initial_window_size: Some(Vec2 { x: 640., y: 320. }),
            initial_window_pos: Some(Pos2 { x, y }),
            ..eframe::NativeOptions::default()
        },
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());

            Box::new(app::App::new(aggregator, cc.egui_ctx.clone()))
        }),
    );
}
