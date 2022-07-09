#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;

use eframe::egui;
use egui::{FontData, FontDefinitions, FontFamily, Pos2, Vec2};

mod app;
mod config;
mod search;
mod util;

#[cfg(target_os = "windows")]
fn fix_stdout() {
    use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};

    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

fn main() {
    #[cfg(target_os = "windows")]
    fix_stdout();

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

            if let Some(font_path) = &config.style.custom_font {
                let mut fonts = FontDefinitions::default();

                fonts.font_data.insert(
                    "custom_font".to_owned(),
                    FontData::from_owned(fs::read(font_path).unwrap()),
                );

                fonts
                    .families
                    .get_mut(&FontFamily::Proportional)
                    .unwrap()
                    .insert(0, "custom_font".to_owned());
                fonts
                    .families
                    .get_mut(&FontFamily::Monospace)
                    .unwrap()
                    .push("custom_font".to_owned());

                cc.egui_ctx.set_fonts(fonts);
            }

            Box::new(app::App::new(cc.egui_ctx.clone(), config))
        }),
    );
}
