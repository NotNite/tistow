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
            let mut visuals = egui::Visuals::dark();

            // colors
            if let Some(bg_color) = &config.style.bg_color {
                let bg_color = util::hex_to_color32(bg_color);
                visuals.widgets.noninteractive.bg_fill = bg_color;
            }

            if let Some(input_bg_color) = &config.style.input_bg_color {
                let input_bg_color = util::hex_to_color32(input_bg_color);
                visuals.extreme_bg_color = input_bg_color;
            }

            if let Some(hovered_bg_color) = &config.style.hovered_bg_color {
                let hovered_bg_color = util::hex_to_color32(hovered_bg_color);
                visuals.widgets.hovered.bg_fill = hovered_bg_color;
            }

            if let Some(selected_bg_color) = &config.style.selected_bg_color {
                let selected_bg_color = util::hex_to_color32(selected_bg_color);
                visuals.widgets.active.bg_fill = selected_bg_color;
            }

            if let Some(text_color) = &config.style.text_color {
                let text_color = util::hex_to_color32(text_color);
                visuals.override_text_color = Some(text_color);
            }

            if let Some(stroke_color) = &config.style.stroke_color {
                let stroke_color = util::hex_to_color32(stroke_color);
                visuals.selection.stroke.color = stroke_color; // text input
                visuals.widgets.hovered.bg_stroke.color = stroke_color; // hover
                visuals.widgets.active.bg_stroke.color = stroke_color; // selection
            }

            cc.egui_ctx.set_visuals(visuals);

            // fonts
            if let Some(font_path) = &config.style.font {
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
