use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct Entry {
    pub original_path: String,
    pub stored_path: String,
    pub deleted_at: String,
}

fn index_path() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("~/.local/share"));
    base.join("riptide").join("index.json")
}

pub fn add_entry(original: &PathBuf, stored: &PathBuf) -> anyhow::Result<()> {
    let mut entries = load_entries().unwrap_or_default();
    let entry = Entry {
        original_path: original.display().to_string(),
        stored_path: stored.display().to_string(),
        deleted_at: chrono::Utc::now().to_rfc3339(),
    };
    entries.push(entry);
    let json = serde_json::to_string_pretty(&entries)?;
    fs::create_dir_all(index_path().parent().unwrap())?;
    fs::write(index_path(), json)?;
    Ok(())
}

pub fn load_entries() -> anyhow::Result<Vec<Entry>> {
    if index_path().exists() {
        let data = fs::read_to_string(index_path())?;
        let entries: Vec<Entry> = serde_json::from_str(&data)?;
        Ok(entries)
    } else {
        Ok(vec![])
    }
}
