use std::process::Command;

use arboard::Clipboard;
use device_query::DeviceState;
use egui::Key;

use crate::config::Config;
use crate::search::{ResultAction, Search, SearchResult};
use crate::util::{get_shortcuts, is_hotkey_pressed};

pub struct App {
    input: String,
    device_state: DeviceState,
    search: Search,
    config: Config,

    focused: i32,
    items: u32,
}

impl App {
    pub fn new(config: Config) -> Self {
        let shortcuts = get_shortcuts(&config);
        let search = Search::new(shortcuts);

        Self {
            input: String::default(),
            device_state: DeviceState::new(),
            search,
            config,

            focused: -1,
            items: 0,
        }
    }

    fn cycle_focus(&mut self) {
        self.focused += 1;
        if self.focused >= self.items.try_into().unwrap() {
            self.focused = -1;
        }
    }

    fn handle_select(&mut self, selection: &SearchResult) -> bool {
        println!("select: {}", selection.text);
        if let Some(action) = &selection.action {
            let should_close = match action {
                ResultAction::Open { path } => {
                    println!("open: {}", path);
                    Command::new("explorer")
                        .arg(path)
                        .spawn()
                        .expect("couldn't spawn process");

                    true
                }
                ResultAction::Copy { text } => {
                    let mut clipboard = Clipboard::new().unwrap();
                    clipboard
                        .set_text(text.to_string())
                        .expect("couldn't copy to clipboard");

                    false
                }
            };

            if should_close {
                self.input = String::default();
                self.focused = -1;
                return true;
            }
        }

        false
    }
}

impl eframe::App for App {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        //self.exited = true;
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let results = self.search.search(&self.input);
        self.items = results.len().try_into().unwrap();

        //println!("{}", self.focused);

        if ctx.input().key_pressed(egui::Key::Tab) {
            self.cycle_focus()
        }

        if ctx.input().key_down(Key::Escape) {
            _frame.set_visibility(false);
        }

        // global hotkeys
        if is_hotkey_pressed(&self.device_state, &self.config.general.hotkey) {
            self.input = String::default();
            self.focused = -1;
            _frame.set_visibility(true);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let input_widget = egui::TextEdit::singleline(&mut self.input)
                .hint_text("search anything...")
                .lock_focus(true);
            let input_res = ui.add_sized((ui.available_width(), 18_f32), input_widget);

            // user presses enter in the input field (select first input)
            if input_res.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
                if !results.is_empty() {
                    let result = &results[0];

                    let should_close = self.handle_select(result);
                    if should_close {
                        _frame.set_visibility(false);
                    }
                }
            // user selects option manually
            } else if self.focused != -1 && ui.input().key_pressed(egui::Key::Enter) {
                let result = &results[self.focused as usize];

                let should_close = self.handle_select(result);
                if should_close {
                    _frame.set_visibility(false);
                }
            }

            if self.focused == -1 {
                input_res.request_focus();
            }

            ui.separator();

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .min_scrolled_width(ui.available_width())
                .show(ui, |scroll_ui| {
                    if self.input.is_empty() {
                        return;
                    }

                    if self.input.trim() == "anything" {
                        scroll_ui.label("...uh, not like that");
                    }

                    for (pos, result) in results.iter().enumerate() {
                        let label_res = scroll_ui.selectable_label(false, &result.text);

                        if self.focused == pos.try_into().unwrap() {
                            label_res.request_focus();
                            label_res.scroll_to_me(None);
                        }

                        if label_res.clicked() {
                            let should_close = self.handle_select(result);
                            if should_close {
                                _frame.set_visibility(false);
                            }
                        }
                    }
                });
        });
    }
}
