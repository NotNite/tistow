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

struct KeyMatch {
    path: PathBuf,
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
        let alias = self.aliases.get(input.trim());

        let shortcuts = self.shortcuts.clone();
        let shortcuts: Vec<Option<&PathBuf>> = shortcuts.iter().map(Some).collect();

        // we need to prioritize, in order:
        // - aliases
        // - exact matches
        // - starts with input
        // - everything else
        let mut available_shortcuts: Vec<KeyMatch> = shortcuts
            .iter()
            .map(|path| {
                let path = path.unwrap().to_path_buf();
                let name = path.file_stem().unwrap().to_str().unwrap().to_string();

                let alias_matches = alias.is_some() && name.trim() == alias.unwrap().trim();
                let exact_match = name.trim().to_lowercase() == input.trim().to_lowercase();
                let starts_with = name
                    .trim()
                    .to_lowercase()
                    .starts_with(&input.trim().to_lowercase());
                let fuzzy = self
                    .matcher
                    .fuzzy_match(&name.to_lowercase(), &input.to_lowercase())
                    .is_some();

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
                    kind: match_kind,
                }
            })
            .filter(|x| x.kind.is_some())
            .collect();

        available_shortcuts.sort_by_cached_key(|x| x.kind);

        let results = available_shortcuts
            .iter()
            .map(|k| {
                let path = &k.path;
                let name = path.file_stem().unwrap().to_str().unwrap().to_string();

                SearchResult {
                    mode: SearchMode::Search,
                    text: name,
                    action: Some(ResultAction::Open {
                        path: path.to_owned(),
                    }),
                }
            })
            .collect();

        results
    }
}
