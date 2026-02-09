use anyhow::Result;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use super::types::HistoryMessage;

pub const MAX_HISTORY_MESSAGES: usize = 50;

pub fn context_dir() -> String {
    env::var("STRIVE_CONTEXT_PATH").unwrap_or_else(|_| "./context".to_string())
}

pub fn get_history_path(user_id: &str, channel_id: &str) -> PathBuf {
    let dir = context_dir();
    fs::create_dir_all(&dir).ok();
    PathBuf::from(dir).join(format!("{}_{}.jsonl", channel_id, user_id))
}

pub fn load_history(path: &Path) -> Result<Vec<HistoryMessage>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut messages = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        if let Ok(msg) = serde_json::from_str::<HistoryMessage>(&line) {
            messages.push(msg);
        }
    }

    Ok(messages)
}

pub fn save_history(path: &Path, messages: &[HistoryMessage]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;

    for msg in messages {
        serde_json::to_writer(&mut file, msg)?;
        writeln!(file)?;
    }

    Ok(())
}
