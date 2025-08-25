use anyhow::{Context, Result};
use base64::Engine;
use fs_err as fs;
use rand::{RngCore, rng}; // rand 0.9
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use rustix::fs::{Mode, OFlags, open};
#[cfg(unix)]
use std::os::unix::fs::symlink;

fn rand_suffix() -> String {
    let mut b = [0u8; 6];
    rng().fill_bytes(&mut b); // rand 0.9
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b)
}

/// Fsync d’un répertoire (durabilité des entrées)
fn fsync_dir(dir: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let f = open(dir, OFlags::RDONLY | OFlags::DIRECTORY, Mode::empty())
            .with_context(|| format!("open(dir) for fsync: {}", dir.display()))?;
        rustix::fs::fdatasync(&f).context("fdatasync(dir)")?;
        Ok(())
    }
    #[cfg(windows)]
    {
        let _ = dir;
        Ok(())
    }
}

/// rendu visible pour graveyard.rs
pub(crate) fn is_exdev(err: &std::io::Error) -> bool {
    match err.raw_os_error() {
        #[cfg(unix)]
        Some(n) => n == libc::EXDEV,
        #[cfg(windows)]
        Some(_n) => false,
        None => false,
    }
}

fn copy_file_fsync(src: &Path, dst: &Path) -> Result<()> {
    let mut in_f = fs::File::open(src).with_context(|| format!("open(src): {}", src.display()))?;
    let mut out_f =
        fs::File::create(dst).with_context(|| format!("create(dst): {}", dst.display()))?;
    std::io::copy(&mut in_f, &mut out_f).context("copy stream")?;
    out_f.sync_all().context("fsync(dst)")?;
    Ok(())
}

pub fn copy_recursively(src: &Path, dst: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(src)?;
    if meta.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            copy_recursively(&entry.path(), &dst.join(entry.file_name()))?;
        }
        Ok(())
    } else if meta.file_type().is_symlink() {
        #[cfg(unix)]
        {
            let target = fs::read_link(src)?;
            symlink(target, dst)?;
            Ok(())
        }
        #[cfg(windows)]
        {
            let target = fs::read_link(src)?;
            let real = if target.is_absolute() {
                target
            } else {
                src.parent().unwrap_or(Path::new(".")).join(target)
            };
            copy_recursively(&real, dst)
        }
    } else {
        copy_file_fsync(src, dst)
    }
}

pub fn remove_recursively(p: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(p)?;
    if meta.is_dir() && !meta.file_type().is_symlink() {
        for entry in fs::read_dir(p)? {
            let entry = entry?;
            remove_recursively(&entry.path())?;
        }
        fs::remove_dir(p)?;
    } else {
        fs::remove_file(p)?;
    }
    Ok(())
}

pub fn safe_move_unique(src: &Path, dst_dir: &Path, basename: &OsString) -> Result<PathBuf> {
    fs::create_dir_all(dst_dir).with_context(|| format!("create_dir_all {}", dst_dir.display()))?;
    let ts = chrono::Local::now().format("%Y%m%dT%H%M%S");
    let unique = format!(
        "{}__{}__{}",
        ts,
        rand_suffix(),
        Path::new(basename).to_string_lossy()
    );
    let dst = dst_dir.join(unique);

    match fs::rename(src, &dst) {
        Ok(()) => {
            fsync_dir(dst_dir)?;
            Ok(dst)
        }
        Err(e) if is_exdev(&e) => {
            let tmp = dst.with_extension("copying");
            copy_recursively(src, &tmp)?;
            fsync_dir(tmp.parent().unwrap_or(dst_dir))?;
            fs::rename(&tmp, &dst).context("swap tmp->dst")?;
            fsync_dir(dst_dir)?;
            remove_recursively(src)?;
            Ok(dst)
        }
        Err(e) => Err(e).with_context(|| format!("rename {} -> {}", src.display(), dst.display())),
    }
}
