use anyhow::{Context, Result};
use directories::BaseDirs;
use std::path::PathBuf;

pub fn config_path() -> Result<PathBuf> {
    let base = BaseDirs::new().context("unable to resolve home directory")?;
    Ok(base.config_dir().join("ai").join("config.toml"))
}

pub fn history_path() -> Result<PathBuf> {
    let base = BaseDirs::new().context("unable to resolve home directory")?;
    Ok(base.cache_dir().join("ai").join("history.json"))
}
