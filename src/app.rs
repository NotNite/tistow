use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{self, Arc};

use anyhow::Context;
use arboard::Clipboard;
use egui::Key;
use mlua::Lua;

use crate::config::{get_scripts, Config};
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
    lua: Arc<mlua::Lua>,
}

impl App {
    pub fn new(ctx: egui::Context, config: Config) -> Self {
        let shortcuts = get_shortcuts(&config);
        let mut search = Search::new(shortcuts, config.search.aliases.clone());

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

        // some very ugly wip lua code
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
        for (shortcut, _) in custom_shortcuts {
            search.add_custom_shortcut(shortcut);
        }

        Self {
            search,
            state: AppState::First,
            _hotkey_thread: hotkey_thread,
            events_rx,
            _config: config,
            lua: Arc::new(lua),
        }
    }

    fn handle_select(selection: &SearchResult, lua: &Arc<Lua>) -> anyhow::Result<bool> {
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
                let custom_shortcuts: HashMap<String, mlua::Function> =
                    lua.named_registry_value("custom_shortcuts").unwrap();

                let func = custom_shortcuts.get(&selection.text).unwrap();

                func.call::<_, bool>(()).unwrap()
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
                Self::draw_opened_central(ui, opened, results, &self.lua)
            })
            .inner
    }

    fn draw_opened_central(
        ui: &mut egui::Ui,
        mut opened: Opened,
        results: Vec<SearchResult>,
        lua: &Arc<Lua>,
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
                if Self::handle_select(result, lua)? {
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
                        let should_close = Self::handle_select(result, lua)?;
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

        let state = self.get_new_state(ctx);
        self.set_state(state.unwrap(), frame);
    }
}
