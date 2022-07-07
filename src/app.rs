use std::str::FromStr;
use std::sync;

use anyhow::Context;
use arboard::Clipboard;
use egui::Key;

use crate::config::Config;
use crate::search::{ResultAction, Search, SearchResult};
use crate::util::get_shortcuts;

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
    search: Search,
    state: AppState,
    _hotkey_thread: std::thread::JoinHandle<()>,
    events_rx: sync::mpsc::Receiver<HotkeyEvent>,
    _config: Config,
}

impl App {
    pub fn new(ctx: egui::Context, config: Config) -> Self {
        let shortcuts = get_shortcuts(&config);
        let search = Search::new(shortcuts);

        let (events_tx, events_rx) = sync::mpsc::channel();
        let hotkey_thread = std::thread::spawn({
            let config = config.clone();
            let hotkeys: Vec<_> = config
                .general
                .hotkey
                .iter()
                .map(|hk| device_query::Keycode::from_str(hk).unwrap())
                .collect();

            move || {
                let device_state = device_query::DeviceState::new();

                loop {
                    // global hotkeys
                    if crate::util::is_hotkey_pressed(&device_state, &hotkeys) {
                        events_tx.send(HotkeyEvent::Open).unwrap();
                        ctx.request_repaint();
                    }

                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        });

        Self {
            search,
            state: AppState::First,
            _hotkey_thread: hotkey_thread,
            events_rx,
            _config: config,
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
            AppState::Opened(opened) => self.process_opened(opened, ctx),
        }
    }

    fn process_opened(&self, opened: &Opened, ctx: &egui::Context) -> anyhow::Result<AppState> {
        let mut opened = opened.clone();
        let results = self.search.search(&opened.input);
        opened.items = results.len();

        //println!("{}", self.focused);
        if ctx.input().key_pressed(egui::Key::Tab) {
            opened.cycle_focus();
        }
        if ctx.input().key_released(Key::Escape) {
            return Ok(AppState::Unopened);
        }

        egui::CentralPanel::default()
            .show(ctx, |ui| Self::draw_opened_central(ui, opened, results))
            .inner
    }

    fn draw_opened_central(
        ui: &mut egui::Ui,
        mut opened: Opened,
        results: Vec<SearchResult>,
    ) -> anyhow::Result<AppState> {
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

        let inner = egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .min_scrolled_width(ui.available_width())
            .show(ui, |scroll_ui| {
                if opened.input.is_empty() {
                    return Ok(None);
                }

                if opened.input.trim() == "anything" && results.is_empty() {
                    scroll_ui.label("...uh, not like that");
                }

                for (pos, result) in results.iter().enumerate() {
                    let label_res = scroll_ui.selectable_label(false, &result.text);

                    if opened.focused == Some(pos) {
                        label_res.request_focus();
                        label_res.scroll_to_me(None);
                    }

                    if label_res.clicked() {
                        let should_close = Self::handle_select(result)?;
                        if should_close {
                            return Ok(Some(AppState::Unopened));
                        }
                    }
                }

                anyhow::Ok(None)
            })
            .inner?;

        Ok(inner.unwrap_or(AppState::Opened(opened)))
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
