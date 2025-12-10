#![allow(clippy::map_entry)]
use globset::Glob;
use grep::{
    matcher::Matcher,
    regex::{RegexMatcher, RegexMatcherBuilder},
    searcher::{sinks::Lossy, MmapChoice, SearcherBuilder},
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Mutex};
use tauri::{Emitter, EventTarget};
use zouni::Dirent;

static CANCEL: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));
const GREP_EVENT_NAME: &str = "grep_progress";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GrepProgress {
    processing: String,
    current: usize,
    total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepRequest {
    pub condition: String,
    pub start_directory: String,
    pub file_type: String,
    pub match_by_word: bool,
    pub case_sensitive: bool,
    pub regexp: bool,
    pub recursive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GrepResult {
    full_path: String,
    line_number: u64,
    line: String,
    ranges: Vec<(usize, usize)>,
}

pub async fn run_grep(window: &tauri::WebviewWindow, e: GrepRequest) -> Result<Vec<GrepResult>, String> {
    {
        let mut token = CANCEL.lock().unwrap();
        *token = false;
    }
    let items = zouni::fs::readdir(e.start_directory, e.recursive, false)?;
    let glob = Glob::new(&e.file_type).map_err(|e| e.to_string())?.compile_matcher();
    let files: Vec<Dirent> = items
        .into_iter()
        .filter(|item| !item.attributes.is_symbolic_link && !item.attributes.is_directory && !item.attributes.is_device && !item.attributes.is_system)
        .filter(|item| {
            if e.file_type == "*.*" {
                true
            } else {
                glob.is_match(&item.name)
            }
        })
        .collect();

    if files.is_empty() {
        return Ok(Vec::new());
    };
    let total = files.len();

    let mut builder = RegexMatcherBuilder::new();
    let builder = builder.case_insensitive(!e.case_sensitive);
    let builder = builder.word(e.match_by_word);
    let builder = builder.fixed_strings(!e.regexp);
    let matcher: RegexMatcher = builder.build(&e.condition).unwrap();
    let mut searcher = SearcherBuilder::new().memory_map(unsafe { MmapChoice::auto() }).build();

    let mut results: HashMap<String, GrepResult> = HashMap::new();
    for (count, file) in files.iter().enumerate() {
        if let Ok(token) = CANCEL.try_lock() {
            if *token {
                break;
            }
        }
        window
            .emit_to(
                EventTarget::WebviewWindow {
                    label: window.label().to_string(),
                },
                GREP_EVENT_NAME,
                GrepProgress {
                    processing: file.full_path.clone(),
                    current: count + 1,
                    total,
                },
            )
            .unwrap();

        searcher
            .search_path(
                &matcher,
                &file.full_path,
                Lossy(|line_number, line| {
                    matcher.find_iter(line.as_bytes(), |matched| {
                        let full_path = file.full_path.clone();
                        let mut key = full_path.clone();
                        key.push_str(&line_number.to_string());
                        if results.contains_key(&key) {
                            results.get_mut(&key).unwrap().ranges.push((matched.start(), matched.end()));
                        } else {
                            let result = GrepResult {
                                full_path,
                                line_number,
                                line: line.to_string(),
                                ranges: vec![(matched.start(), matched.end())],
                            };
                            results.insert(key, result);
                        }

                        true
                    })?;

                    Ok(true)
                }),
            )
            .map_err(|e| e.to_string())?;
    }

    Ok(results.into_values().collect())
}

pub fn cancel() {
    loop {
        if let Ok(mut token) = CANCEL.try_lock() {
            *token = true;
            break;
        }
    }
}
