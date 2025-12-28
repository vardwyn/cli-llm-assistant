use crate::paths::history_path;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub timestamp: u64,
    pub model: String,
    pub preset_name: String,
    pub user_input: String,
    pub response: String,
}

pub fn append_entry(entry: HistoryEntry, max_entries: usize) -> Result<()> {
    let path = history_path()?;
    ensure_parent(&path)?;
    let mut entries = load_history(&path).unwrap_or_default();
    entries.push(entry);
    if max_entries == 0 {
        return Ok(());
    }
    if entries.len() > max_entries {
        let start = entries.len() - max_entries;
        entries = entries[start..].to_vec();
    }
    let data = serde_json::to_vec_pretty(&entries)?;
    fs::write(&path, data).with_context(|| format!("failed to write history to {}", path.display()))?;
    Ok(())
}

pub fn load_history(path: &Path) -> Result<Vec<HistoryEntry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(path)
        .with_context(|| format!("failed to read history from {}", path.display()))?;
    if data.trim().is_empty() {
        return Ok(Vec::new());
    }
    let entries = serde_json::from_str(&data)
        .with_context(|| format!("failed to parse history from {}", path.display()))?;
    Ok(entries)
}

pub fn clear_history() -> Result<()> {
    let path = history_path()?;
    if path.exists() {
        fs::remove_file(&path)
            .with_context(|| format!("failed to remove history at {}", path.display()))?;
    }
    Ok(())
}

pub fn replay_response(offset: usize) -> Result<String> {
    if offset == 0 {
        bail!("history index must be >= 1");
    }
    let path = history_path()?;
    let entries = load_history(&path)?;
    if entries.is_empty() {
        bail!("history is empty");
    }
    if offset > entries.len() {
        bail!("history only has {} entries", entries.len());
    }
    let index = entries.len() - offset;
    Ok(entries[index].response.clone())
}

pub fn now_timestamp() -> Result<u64> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system time before unix epoch")?;
    Ok(now.as_secs())
}

fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create history directory {}", parent.display()))?;
    }
    Ok(())
}
