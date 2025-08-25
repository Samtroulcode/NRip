use anyhow::{Context, Result};
use fd_lock::RwLock;
use fs_err as fs;
use serde::{Deserialize, Serialize};
use serde_json as json;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Kind {
    File,
    Dir,
    Symlink,
    #[default]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub original_path: PathBuf,
    pub trashed_path: PathBuf,
    pub deleted_at: i64,
    #[serde(default)]
    pub kind: Kind, // ← nouveau champ, défaut = Other pour compat avec anciens index
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Index {
    pub items: Vec<Entry>,
}

fn index_paths() -> Result<(PathBuf, PathBuf, PathBuf)> {
    let data_dir = crate::paths::data_dir()?;
    let idx_dir = data_dir.clone();
    let gy_dir = data_dir.join("graveyard");
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
    let lock = RwLock::new(lockf);
    let _guard = lock.read().context("lock index for read")?;
    if !idx.exists() {
        return Ok(Index::default());
    }
    let data = fs::read(&idx).with_context(|| format!("read {}", idx.display()))?;
    let index: Index =
        json::from_slice(&data).with_context(|| format!("parse {}", idx.display()))?;
    Ok(index)
}

// API legacy, optionnelle
#[cfg(feature = "legacy_api")]
#[allow(dead_code)]
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

// API legacy, optionnelle
#[cfg(feature = "legacy_api")]
#[allow(dead_code)]
pub fn save_entries(entries: &[Entry]) -> Result<()> {
    let mut idx = load_index().unwrap_or_default();
    idx.items = entries.to_vec();
    save_index(&idx)
}

pub fn basename_of_original(e: &Entry) -> String {
    e.original_path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub fn with_index_mut<F, T>(mut f: F) -> Result<T>
where
    F: FnMut(&mut Index) -> Result<T>,
{
    let (idx_path, dir, _) = index_paths()?;
    fs::create_dir_all(&dir)?;

    // Un seul lock pour la durée de vie de la transaction
    let lockf = fs::File::create(lock_path(&dir)).context("create lock file")?;
    let mut lock = RwLock::new(lockf);
    let _guard = lock.write().context("lock index for update")?;

    // read (si existe)
    let mut idx = if idx_path.exists() {
        let data = fs::read(&idx_path).with_context(|| format!("read {}", idx_path.display()))?;
        json::from_slice(&data).with_context(|| format!("parse {}", idx_path.display()))?
    } else {
        Index::default()
    };

    // user mutation
    let out = f(&mut idx)?;

    // write atomique
    let mut tmp = NamedTempFile::new_in(&dir).context("mkstemp in index dir")?;
    let buf = serde_json::to_vec_pretty(&idx).context("serialize index")?;
    tmp.write_all(&buf).context("write tmp")?;
    tmp.as_file().sync_all().context("fsync tmp")?;
    let tmp_path = tmp.into_temp_path();
    tmp_path.persist(&idx_path).map_err(|e| {
        anyhow::anyhow!(
            "rename {} -> {}: {}",
            e.path.display(),
            idx_path.display(),
            e.error
        )
    })?;

    #[cfg(unix)]
    {
        use rustix::fs::{Mode, OFlags, open};
        let df = open(&dir, OFlags::RDONLY | OFlags::DIRECTORY, Mode::empty())?;
        rustix::fs::fdatasync(&df)?;
    }
    Ok(out)
}
