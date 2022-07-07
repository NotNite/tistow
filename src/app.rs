use std::sync;

use anyhow::Context;
use arboard::Clipboard;
use egui::Key;

use crate::search::{ResultAction, Search, SearchResult};

#[derive(Clone, Copy, Debug)]
pub enum HotkeyEvent {
    Open,
}

#[derive(Default, Clone, Debug)]
struct Opened {
    input: String,
    focused: Option<usize>,
    items: usize,
}
impl Opened {
    pub fn cycle_focus(&mut self) {
        self.focused = match self.focused {
            Some(n) => {
                if n + 1 == self.items {
                    None
                } else {
                    Some(n + 1)
                }
            }
            None => Some(0),
        }
    }
}

#[derive(Clone, Debug)]
enum AppState {
    First,
    Unopened,
    Opened(Opened),
}

pub struct App {
    aggregator: Search,
    state: AppState,
    _hotkey_thread: std::thread::JoinHandle<()>,
    events_rx: sync::mpsc::Receiver<HotkeyEvent>,
}

impl App {
    pub fn new(aggregator: Search, ctx: egui::Context) -> Self {
        let (events_tx, events_rx) = sync::mpsc::channel();
        let hotkey_thread = std::thread::spawn({
            move || {
                let device_state = device_query::DeviceState::new();
                loop {
                    // global hotkeys
                    if crate::util::is_hotkey_pressed(&device_state) {
                        events_tx.send(HotkeyEvent::Open).unwrap();
                        ctx.request_repaint();
                    }

                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        });

        Self {
            aggregator,
            state: AppState::First,
            _hotkey_thread: hotkey_thread,
            events_rx,
        }
    }

    fn handle_select(selection: &SearchResult) -> anyhow::Result<bool> {
        println!("select: {}", selection.text);
        let action = match &selection.action {
            Some(action) => action,
            _ => return Ok(false),
        };

        let should_close = match action {
            ResultAction::Open { path } => {
                open::that(path).context("couldn't spawn process")?;

                true
            }
            ResultAction::Copy { text } => {
                Clipboard::new()?
                    .set_text(text.to_string())
                    .context("couldn't copy to clipboard")?;

                false
            }
        };
        Ok(should_close)
    }

    fn get_new_state(&self, ctx: &egui::Context) -> anyhow::Result<AppState> {
        match &self.state {
            AppState::First => Ok(AppState::Unopened),
            AppState::Unopened => Ok(AppState::Unopened),
            AppState::Opened(opened) => {
                let mut opened = opened.clone();
                let results = self.aggregator.search(&opened.input);
                opened.items = results.len();

                //println!("{}", self.focused);

                if ctx.input().key_pressed(egui::Key::Tab) {
                    opened.cycle_focus();
                }

                if ctx.input().key_released(Key::Escape) {
                    return Ok(AppState::Unopened);
                }

                egui::CentralPanel::default()
                    .show(ctx, |ui| {
                        let input_widget = egui::TextEdit::singleline(&mut opened.input)
                            .hint_text("search anything...")
                            .lock_focus(true);
                        let input_res = ui.add_sized((ui.available_width(), 18_f32), input_widget);

                        if ui.input().key_pressed(egui::Key::Enter) && !results.is_empty() {
                            let result = if input_res.lost_focus() {
                                // user presses enter in the input field (select first input)
                                Some(&results[0])
                            } else if opened.focused.is_some() {
                                // user selects option manually
                                Some(&results[opened.focused.unwrap()])
                            } else {
                                None
                            };

                            if let Some(result) = result {
                                if Self::handle_select(result)? {
                                    return Ok(AppState::Unopened);
                                }
                            }
                        }

                        if opened.focused.is_none() {
                            input_res.request_focus();
                        }

                        ui.separator();

                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .min_scrolled_width(ui.available_width())
                            .show(ui, |scroll_ui| {
                                if opened.input.is_empty() {
                                    return Ok(());
                                }

                                for (pos, result) in results.iter().enumerate() {
                                    let label_res = scroll_ui.selectable_label(false, &result.text);
                                    label_res.enabled();
                                    //label_res.request_focus();

                                    if opened.focused == Some(pos) {
                                        label_res.request_focus();
                                        label_res.scroll_to_me(None);
                                    }
                                }

                                anyhow::Ok(())
                            })
                            .inner?;

                        Ok(AppState::Opened(opened))
                    })
                    .inner
            }
        }
    }

    fn set_state(&mut self, state: AppState, frame: &mut eframe::Frame) {
        self.state = state;
        match &self.state {
            AppState::First => panic!("should never enter first state"),
            AppState::Unopened => {
                frame.set_visibility(false);
            }
            AppState::Opened(_) => {
                frame.set_visibility(true);
            }
        }
    }
}

impl eframe::App for App {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        //self.exited = true;
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let events: Vec<_> = self.events_rx.try_iter().collect();
        for event in events {
            match event {
                HotkeyEvent::Open => self.set_state(AppState::Opened(Opened::default()), frame),
            }
        }
        self.set_state(self.get_new_state(ctx).unwrap(), frame);
    }
}
