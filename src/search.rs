use crate::util::get_shortcuts;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

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
    Open { path: String },
    Copy { text: String },
}

pub struct Search {
    matcher: SkimMatcherV2,
}

impl Search {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
        }
    }

    pub fn search(&mut self, input: &str) -> Vec<SearchResult> {
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

    fn mode_calculator(&mut self, input: &str) -> Vec<SearchResult> {
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

    fn mode_search(&mut self, input: &str) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = Vec::new();
        let options = get_shortcuts();

        for option in options {
            let name = option
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if self.matcher.fuzzy_match(name, input).is_some() {
                results.push(SearchResult {
                    mode: SearchMode::Search,
                    text: name.to_string(),
                    action: Some(ResultAction::Open {
                        path: option.to_str().unwrap_or_default().to_string(),
                    }),
                });
            }
        }

        results
    }
}
