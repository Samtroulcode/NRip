use anyhow::{Context, Result};
use std::path::PathBuf;

pub fn data_dir() -> Result<PathBuf> {
    let base = dirs::data_dir().context("XDG data dir not found")?;
    Ok(base.join("nrip"))
}
