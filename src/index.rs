use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Entry {
    #[serde(default)] // compat: anciennes entrées sans id
    pub id: Option<String>, // id court pour ciblage/restore
    pub original_path: String,
    pub stored_path: String,
    pub deleted_at: String,
}

fn index_path() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("~/.local/share"));
    base.join("riptide").join("index.json")
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

pub fn save_entries(entries: &Vec<Entry>) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(entries)?;
    fs::create_dir_all(index_path().parent().unwrap())?;
    fs::write(index_path(), json)?;
    Ok(())
}

pub fn add_entry(original: &PathBuf, stored: &PathBuf) -> anyhow::Result<()> {
    let mut entries = load_entries().unwrap_or_default();
    let id = Some(crate::graveyard::short_id());
    let entry = Entry {
        id,
        original_path: original.display().to_string(),
        stored_path: stored.display().to_string(),
        deleted_at: chrono::Utc::now().to_rfc3339(),
    };
    entries.push(entry);
    save_entries(&entries)
}

pub fn basename_of_original(e: &Entry) -> String {
    Path::new(&e.original_path)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| e.original_path.clone())
}
