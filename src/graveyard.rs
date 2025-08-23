use std::fs;
use std::path::PathBuf;

use crate::index;

/// Path to the graveyard directory (~/.local/share/riptide/graveyard)
fn graveyard_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("~/.local/share"));
    base.join("riptide").join("graveyard")
}

/// Bury (move) files into the graveyard and update index
pub fn bury(paths: Vec<PathBuf>) -> anyhow::Result<()> {
    let gy = graveyard_dir();
    fs::create_dir_all(&gy)?;

    for path in paths {
        if path.exists() {
            let file_name = path.file_name().unwrap();
            let dest = gy.join(file_name);
            fs::rename(&path, &dest)?;
            index::add_entry(&path, &dest)?;
            println!("moved {} -> {}", path.display(), dest.display());
        } else {
            eprintln!("{} not found", path.display());
        }
    }

    Ok(())
}

/// List files from index
pub fn list() -> anyhow::Result<()> {
    let entries = index::load_entries()?;
    for e in entries {
        println!("{}  {}", e.deleted_at, e.original_path);
    }
    Ok(())
}
