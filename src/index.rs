use anyhow::{Context, Result};
use fd_lock::RwLock;
use fs_err as fs;
use serde::{Deserialize, Serialize};
use serde_json as json;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub original_path: PathBuf,
    pub trashed_path: PathBuf,
    pub deleted_at: i64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Index {
    pub items: Vec<Entry>,
}

fn index_paths() -> Result<(PathBuf, PathBuf, PathBuf)> {
    let data_dir = crate::paths::data_dir()?; // adapte si besoin
    let idx_dir = data_dir.join("nrip"); // ou data_dir direct si déjà …/nrip
    let gy_dir = idx_dir.join("graveyard");
    let idx = idx_dir.join("index.json");
    fs::create_dir_all(&gy_dir)?;
    Ok((idx, idx_dir, gy_dir))
}

fn lock_path(dir: &Path) -> PathBuf {
    dir.join(".index.lock")
}

pub fn load_index() -> Result<Index> {
    let (idx, dir, _) = index_paths()?;
    fs::create_dir_all(&dir)?;
    let lockf = fs::File::create(lock_path(&dir)).context("create lock file")?;
    let mut lock = RwLock::new(lockf); // garder la valeur
    let _guard = lock.write().context("lock index for read")?;

    if !idx.exists() {
        return Ok(Index::default());
    }
    let data = fs::read(&idx).with_context(|| format!("read {}", idx.display()))?;
    let index: Index =
        json::from_slice(&data).with_context(|| format!("parse {}", idx.display()))?;
    Ok(index)
}

pub fn save_index(idx: &Index) -> Result<()> {
    let (idx_path, dir, _) = index_paths()?;
    fs::create_dir_all(&dir)?;
    let lockf = fs::File::create(lock_path(&dir)).context("create lock file")?;
    let mut lock = RwLock::new(lockf);
    let _guard = lock.write().context("lock index for write")?;

    let mut tmp = NamedTempFile::new_in(&dir).context("mkstemp in index dir")?;
    let buf = serde_json::to_vec_pretty(idx).context("serialize index")?;
    tmp.write_all(&buf).context("write tmp")?;
    tmp.as_file().sync_all().context("fsync tmp")?;
    let tmp_path = tmp.into_temp_path();
    tmp_path.persist(&idx_path).map_err(|e| {
        anyhow::anyhow!(
            "rename {} -> {}: {}",
            e.path.display(), // <- anciennement e.file.path().display()
            idx_path.display(),
            e.error
        )
    })?;

    #[cfg(unix)]
    {
        use rustix::fs::{Mode, OFlags, open};
        let df = open(&dir, OFlags::RDONLY | OFlags::DIRECTORY, Mode::empty())?;
        rustix::fs::fdatasync(&df)?; // fdatasync
    }
    Ok(())
}

/* ——— Shims de compat pour ton code actuel ——— */

pub fn load_entries() -> Result<Vec<Entry>> {
    Ok(load_index()?.items)
}

pub fn save_entries(entries: &Vec<Entry>) -> Result<()> {
    let mut idx = load_index().unwrap_or_default();
    idx.items = entries.clone();
    save_index(&idx)
}

pub fn basename_of_original(e: &Entry) -> String {
    e.original_path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}
