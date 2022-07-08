#![allow(clippy::if_same_then_else)]
use figment::value::Map;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use std::path::PathBuf;

pub enum SearchMode {
    Search,
    Calculator,
}

pub struct SearchResult {
    pub mode: SearchMode,
    pub text: String,
    pub action: Option<ResultAction>,
}

pub enum ResultAction {
    Open { path: PathBuf },
    Copy { text: String },
}

pub struct Search {
    matcher: SkimMatcherV2,
    shortcuts: Vec<PathBuf>,
    aliases: Map<String, String>,
}

impl Search {
    pub fn new(shortcuts: Vec<PathBuf>, aliases: Map<String, String>) -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
            shortcuts,
            aliases,
        }
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

    fn mode_search(&self, input: &str) -> Vec<SearchResult> {
        let shortcuts = self.shortcuts.iter().cloned();
        let mut results: Vec<SearchResult> = Vec::new();

        let alias = self.aliases.get(input.trim());
        let mut idx = 0;

        // this is really cursed but i can't figure out a good way to sort this
        // we need to prioritize, in order:
        // - aliases
        // - exact matches
        // - starts with input
        // - everything else

        // aliases
        for path in shortcuts.clone() {
            let name = path.file_stem().unwrap().to_str().unwrap().to_string();
            let alias_matches = alias.is_some() && name.trim() == alias.unwrap().trim();
            let already_added = results.iter().any(|r| r.text == name);

            if alias_matches && !already_added {
                results.insert(
                    idx,
                    SearchResult {
                        mode: SearchMode::Search,
                        text: name.clone(),
                        action: Some(ResultAction::Open { path }),
                    },
                );
                idx += 1;
            }
        }

        // exact matches
        for path in shortcuts.clone() {
            let name = path.file_stem().unwrap().to_str().unwrap().to_string();
            let exact_match = name.trim().to_lowercase() == input.trim().to_lowercase();
            let already_added = results.iter().any(|r| r.text == name);

            if exact_match && !already_added {
                results.insert(
                    idx,
                    SearchResult {
                        mode: SearchMode::Search,
                        text: name.clone(),
                        action: Some(ResultAction::Open { path }),
                    },
                );
                idx += 1;
            }
        }

        // starts with
        for path in shortcuts.clone() {
            let name = path.file_stem().unwrap().to_str().unwrap().to_string();
            let starts_with = name
                .trim()
                .to_lowercase()
                .starts_with(&input.trim().to_lowercase());
            let already_added = results.iter().any(|r| r.text == name);

            if starts_with && !already_added {
                results.insert(
                    idx,
                    SearchResult {
                        mode: SearchMode::Search,
                        text: name.clone(),
                        action: Some(ResultAction::Open { path }),
                    },
                );
                idx += 1;
            }
        }

        // everything else
        for path in shortcuts.clone() {
            let name = path.file_stem().unwrap().to_str().unwrap().to_string();
            let already_added = results.iter().any(|r| r.text == name);

            if !already_added
                && self
                    .matcher
                    .fuzzy_match(&name.to_lowercase(), &input.to_lowercase())
                    .is_some()
            {
                results.push(SearchResult {
                    mode: SearchMode::Search,
                    text: name.clone(),
                    action: Some(ResultAction::Open { path }),
                });
            }
        }

        results
    }
}
