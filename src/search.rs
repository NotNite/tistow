use std::path::{Path, PathBuf};

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use walkdir::WalkDir;

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
            format!("= {}", n)
        } else {
            "= ERROR".to_string()
        };

        vec![SearchResult {
            mode: SearchMode::Calculator,
            text: res,
            action: None,
        }]
    }

    fn mode_search(&mut self, input: &str) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = Vec::new();

        let start_menu = Path::new(&std::env::var("AppData").unwrap())
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu");

        let options: Vec<PathBuf> = WalkDir::new(start_menu)
            .into_iter()
            .filter(|x| {
                if let Ok(x) = x {
                    let path = x.path().to_str().unwrap();

                    path.to_lowercase().ends_with(".lnk")
                } else {
                    false
                }
            })
            .map(|x| x.unwrap().path().to_owned())
            .collect();

        for option in options {
            let name = option.file_stem().unwrap().to_str().unwrap();
            if self.matcher.fuzzy_match(name, input).is_some() {
                results.push(SearchResult {
                    mode: SearchMode::Search,
                    text: name.to_string(),
                    action: Some(ResultAction::Open {
                        path: option.to_str().unwrap().to_string(),
                    }),
                });
            }
        }

        results
    }
}
