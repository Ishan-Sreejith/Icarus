use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

#[derive(Debug, Default)]
pub struct TrainingData {
    exact: HashMap<String, String>,
    keyword: Vec<(Vec<String>, String)>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum TrainingFile {
    Map(HashMap<String, String>),
    List(Vec<TrainingEntry>),
    Object {
        pairs: Vec<TrainingEntry>,
        #[serde(default)]
        keyword: Vec<TrainingEntry>,
    },
}

#[derive(Debug, Deserialize)]
struct TrainingEntry {
    prompt: String,
    answer: String,
    #[serde(default)]
    keywords: Vec<String>,
}

static TRAINING: OnceLock<TrainingData> = OnceLock::new();

pub fn lookup(prompt: &str) -> Option<String> {
    let data = TRAINING.get_or_init(load_training_data);
    let normalized = normalize(prompt);

    if let Some(ans) = data.exact.get(&normalized) {
        return Some(ans.clone());
    }

    if !normalized.is_empty() {
        for (keywords, ans) in &data.keyword {
            if keywords.iter().all(|k| normalized.contains(k)) {
                return Some(ans.clone());
            }
        }
    }

    None
}

fn load_training_data() -> TrainingData {
    let mut data = TrainingData::default();
    let path = training_path();

    let raw = if let Some(url) = training_url() {
        fetch_url(&url).or_else(|| fs::read_to_string(&path).ok())
    } else {
        fs::read_to_string(&path).ok()
    };

    let raw = match raw {
        Some(s) => s,
        None => return data,
    };

    let parsed: TrainingFile = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return data,
    };

    match parsed {
        TrainingFile::Map(map) => {
            for (k, v) in map {
                data.exact.insert(normalize(&k), v);
            }
        }
        TrainingFile::List(list) => {
            ingest_entries(&mut data, list);
        }
        TrainingFile::Object { pairs, keyword } => {
            ingest_entries(&mut data, pairs);
            ingest_entries(&mut data, keyword);
        }
    }

    data
}

fn ingest_entries(data: &mut TrainingData, entries: Vec<TrainingEntry>) {
    for entry in entries {
        let prompt = normalize(&entry.prompt);
        if prompt.is_empty() {
            continue;
        }

        if entry.keywords.is_empty() {
            data.exact.insert(prompt, entry.answer);
        } else {
            let keywords: Vec<String> = entry
                .keywords
                .into_iter()
                .map(|k| normalize(&k))
                .filter(|k| !k.is_empty())
                .collect();
            if !keywords.is_empty() {
                data.keyword.push((keywords, entry.answer));
            } else {
                data.exact.insert(prompt, entry.answer);
            }
        }
    }
}

fn normalize(input: &str) -> String {
    input
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn training_path() -> PathBuf {
    if let Ok(path) = std::env::var("CORE_TRAINING_DATA") {
        return PathBuf::from(path);
    }
    Path::new("training_data.json").to_path_buf()
}

fn training_url() -> Option<String> {
    std::env::var("CORE_TRAINING_URL").ok()
}

fn fetch_url(url: &str) -> Option<String> {
    let output = Command::new("curl")
        .arg("-fsSL")
        .arg(url)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}
