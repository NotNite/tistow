use std::collections::HashMap;
use std::process::Command;
use std::str::FromStr;
use std::{fs, sync};

use anyhow::Context;
use arboard::Clipboard;
use egui::Key;
use freedesktop_desktop_entry::DesktopEntry;
use mlua::Lua;

use crate::config::{get_scripts, Config};
use crate::search::{ResultAction, Search, SearchResult};
use crate::util::get_shortcuts;

#[derive(Clone, Copy, Debug)]
pub enum HotkeyEvent {
    Open,
}

#[derive(Clone, Debug)]
pub enum LuaShortcutEvent {
    Add(String),
    Done,
}

#[derive(Clone, Debug)]
pub enum LuaEvent {
    RunCallback(String),
    Close,
}

pub struct AppChannels {
    hotkeys_rx: sync::mpsc::Receiver<HotkeyEvent>,
    lua_run_tx: sync::mpsc::Sender<LuaEvent>,
    lua_close_rx: sync::mpsc::Receiver<LuaEvent>,
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
    app_channels: AppChannels,
    _hotkey_thread: std::thread::JoinHandle<()>,
    _config: Config,
}

impl App {
    pub fn new(ctx: egui::Context, config: Config) -> Self {
        let shortcuts = get_shortcuts(&config);
        let mut search = Search::new(shortcuts, config.search.aliases.clone());

        let (events_tx, hotkeys_rx) = sync::mpsc::channel();
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

        let (run_tx, run_rx) = sync::mpsc::channel();
        let (close_tx, close_rx) = sync::mpsc::channel();
        let (shortcuts_tx, shortcuts_rx) = sync::mpsc::channel();

        std::thread::spawn(move || {
            let lua = Lua::new();
            let lua_table = lua.create_table().unwrap();

            lua.set_named_registry_value("custom_shortcuts", HashMap::<String, String>::new())
                .unwrap();

            let open = lua
                .create_function(|_, open: String| -> mlua::Result<()> {
                    open::that(open)?;
                    Ok(())
                })
                .unwrap();
            lua_table.set("open", open).unwrap();

            let copy = lua
                .create_function(|_, text: String| -> mlua::Result<()> {
                    Clipboard::new()
                        .unwrap()
                        .set_text(text)
                        .context("couldn't copy to clipboard")
                        .unwrap();

                    Ok(())
                })
                .unwrap();
            lua_table.set("copy", copy).unwrap();

            let add_entry = lua
                .create_function(
                    |lua, (name, func): (String, mlua::Function)| -> mlua::Result<()> {
                        let mut custom_shortcuts: HashMap<String, mlua::Function> =
                            lua.named_registry_value("custom_shortcuts").unwrap();
                        custom_shortcuts.insert(name, func);
                        lua.set_named_registry_value("custom_shortcuts", custom_shortcuts)
                            .unwrap();

                        Ok(())
                    },
                )
                .unwrap();
            lua_table.set("add_entry", add_entry).unwrap();

            lua.globals().set("tistow", lua_table).unwrap();

            let scripts = get_scripts();
            println!("scripts to load: {}", scripts.len());
            for script in scripts {
                lua.load(&script).exec().unwrap();
            }

            let custom_shortcuts: HashMap<String, mlua::Function> =
                lua.named_registry_value("custom_shortcuts").unwrap();
            for (name, _) in custom_shortcuts {
                shortcuts_tx.send(LuaShortcutEvent::Add(name)).unwrap();
            }
            shortcuts_tx.send(LuaShortcutEvent::Done).unwrap();

            loop {
                match run_rx.recv() {
                    Ok(LuaEvent::RunCallback(callback)) => {
                        let custom_shortcuts: HashMap<String, mlua::Function> =
                            lua.named_registry_value("custom_shortcuts").unwrap();
                        let func = custom_shortcuts.get(&callback).unwrap();
                        let should_close = func.call::<_, bool>(()).unwrap();

                        if should_close {
                            close_tx.send(LuaEvent::Close).unwrap();
                        }
                    }
                    Ok(LuaEvent::Close) => {
                        todo!()
                    }
                    Err(e) => {
                        println!("lua thread error: {}", e);
                    }
                }
            }
        });

        loop {
            match shortcuts_rx.recv() {
                Ok(LuaShortcutEvent::Add(name)) => {
                    search.add_custom_shortcut(name);
                }
                Ok(LuaShortcutEvent::Done) => {
                    break;
                }
                Err(e) => {
                    println!("shortcuts thread error: {}", e);
                }
            }
        }

        Self {
            search,
            state: AppState::First,

            app_channels: AppChannels {
                hotkeys_rx,
                lua_run_tx: run_tx,
                lua_close_rx: close_rx,
            },

            _hotkey_thread: hotkey_thread,
            _config: config,
        }
    }

    fn handle_select(selection: &SearchResult, app_channels: &AppChannels) -> anyhow::Result<bool> {
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
            ResultAction::Lua => {
                // ghelp
                app_channels
                    .lua_run_tx
                    .send(LuaEvent::RunCallback(selection.text.clone()))
                    .unwrap();

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
            .show(ctx, |ui| {
                Self::draw_opened_central(ui, opened, results, &self.app_channels)
            })
            .inner
    }

    fn draw_opened_central(
        ui: &mut egui::Ui,
        mut opened: Opened,
        results: Vec<SearchResult>,
        app_channels: &AppChannels,
    ) -> anyhow::Result<AppState> {
        let input_widget = egui::TextEdit::singleline(&mut opened.input)
            .hint_text("search anything...")
            .lock_focus(true);
        let input_res = ui.add_sized((ui.available_width(), 18_f32), input_widget);

        if app_channels.lua_close_rx.try_recv().is_ok() {
            return Ok(AppState::Unopened);
        }

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
                if Self::handle_select(result, app_channels)? {
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
                    let mut label_res = scroll_ui.selectable_label(false, &result.text);

                    if let Some(ResultAction::Open { path }) = &result.action {
                        label_res = label_res.on_hover_text(path.to_str().unwrap());
                    }

                    if opened.focused == Some(pos) {
                        label_res.request_focus();
                        label_res.scroll_to_me(None);
                    }

                    if label_res.clicked() {
                        let should_close = Self::handle_select(result, app_channels)?;
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
        let events: Vec<_> = self.app_channels.hotkeys_rx.try_iter().collect();
        for event in events {
            match event {
                HotkeyEvent::Open => self.set_state(AppState::Opened(Opened::default()), frame),
            }
        }

        let state = self.get_new_state(ctx);
        self.set_state(state.unwrap(), frame);
    }
}
