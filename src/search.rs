use figment::value::Map;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use std::path::PathBuf;

#[derive(Clone)]
pub enum SearchMode {
    Search,
    Calculator,
}

#[derive(Clone)]
pub struct SearchResult {
    pub mode: SearchMode,
    pub text: String,
    pub action: Option<ResultAction>,
}

#[derive(Clone)]
pub enum ResultAction {
    Open { path: PathBuf },
    Copy { text: String },
    Lua,
}

pub struct Search {
    matcher: SkimMatcherV2,
    shortcuts: Vec<PathBuf>,
    aliases: Map<String, String>,

    custom_shortcuts: Vec<SearchResult>,
}

struct KeyMatch {
    path: Option<PathBuf>,
    name: String,
    kind: Option<MatchKind>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
enum MatchKind {
    Alias,
    Exact,
    StartsWith,
    Fuzzy,
}

impl Search {
    pub fn new(shortcuts: Vec<PathBuf>, aliases: Map<String, String>) -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
            shortcuts,
            aliases,

            custom_shortcuts: Vec::new(),
        }
    }

    pub fn add_custom_shortcut(&mut self, name: String) {
        self.custom_shortcuts.push(SearchResult {
            mode: SearchMode::Search,
            text: name,
            action: Some(ResultAction::Lua),
        });
    }

    pub fn search(&self, input: &str) -> Vec<SearchResult> {
        if input.is_empty() {
            return vec![];
        }

        // TODO: find better way to do this
        let mode = if input.starts_with('=') {
            SearchMode::Calculator
        } else {
            SearchMode::Search
        };

        match mode {
            SearchMode::Calculator => self.mode_calculator(&input[1..]),
            SearchMode::Search => self.mode_search(input),
        }
    }

    fn mode_calculator(&self, input: &str) -> Vec<SearchResult> {
        let r = meval::eval_str(input.trim());

        let res = if let Ok(n) = r {
            n.to_string()
        } else {
            "ERROR".to_string()
        };

        vec![SearchResult {
            mode: SearchMode::Calculator,
            text: format!("= {}", res),
            action: Some(ResultAction::Copy { text: res }),
        }]
    }

    fn do_keymatch(
        name: String,
        path: Option<PathBuf>,
        alias: Option<&String>,
        input: &str,
        fuzzy: bool,
    ) -> KeyMatch {
        let alias_matches = alias.is_some() && name.trim() == alias.unwrap().trim();
        let exact_match = name.trim().to_lowercase() == input.trim().to_lowercase();
        let starts_with = name
            .trim()
            .to_lowercase()
            .starts_with(&input.trim().to_lowercase());

        let match_kind = if alias_matches {
            Some(MatchKind::Alias)
        } else if exact_match {
            Some(MatchKind::Exact)
        } else if starts_with {
            Some(MatchKind::StartsWith)
        } else if fuzzy {
            Some(MatchKind::Fuzzy)
        } else {
            None
        };

        KeyMatch {
            path,
            name,
            kind: match_kind,
        }
    }

    fn mode_search(&self, input: &str) -> Vec<SearchResult> {
        let alias = self.aliases.get(input.trim());

        let shortcuts = self.shortcuts.clone();
        let shortcuts: Vec<Option<&PathBuf>> = shortcuts.iter().map(Some).collect();

        // we need to prioritize, in order:
        // - aliases
        // - exact matches
        // - starts with input
        // - everything else
        let mut vec: Vec<KeyMatch> = Vec::new();
        for shortcut in shortcuts {
            let path = shortcut.unwrap().to_path_buf();
            let name = path.file_stem().unwrap().to_str().unwrap().to_string();

            let fuzzy = self
                .matcher
                .fuzzy_match(&name.to_lowercase(), &input.to_lowercase())
                .is_some();

            let km = Self::do_keymatch(name, Some(path), alias, input, fuzzy);
            vec.push(km);
        }

        for custom_shortcut in &self.custom_shortcuts {
            let name = custom_shortcut.text.clone();
            let fuzzy = self
                .matcher
                .fuzzy_match(&name.to_lowercase(), &input.to_lowercase())
                .is_some();

            let km = Self::do_keymatch(name, None, alias, input, fuzzy);
            vec.push(km);
        }

        let mut available_shortcuts: Vec<&KeyMatch> =
            vec.iter().filter(|x| x.kind.is_some()).collect();

        available_shortcuts.sort_by_cached_key(|x| x.kind);

        available_shortcuts
            .iter()
            .map(|k| {
                let path = &k.path;
                let name = &k.name;

                if let Some(path) = path {
                    SearchResult {
                        mode: SearchMode::Search,
                        text: name.to_string(),
                        action: Some(ResultAction::Open {
                            path: path.to_owned(),
                        }),
                    }
                } else {
                    SearchResult {
                        mode: SearchMode::Search,
                        text: name.to_string(),
                        action: Some(ResultAction::Lua),
                    }
                }
            })
            .collect()
    }
}
