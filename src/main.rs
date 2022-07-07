use eframe::egui;
use egui::{Pos2, Vec2};

mod app;
mod config;
mod search;
mod util;

fn main() {
    let config = config::get_config();
    println!("{:#?}", config);

    eframe::run_native(
        "tistow",
        eframe::NativeOptions {
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
        },
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());

            Box::new(app::App::new(cc.egui_ctx.clone(), config))
        }),
    );
}
