// src/fs_safemove.rs
#![allow(clippy::needless_pass_by_value)]
use anyhow::{Context, Result};
use fs_err as fs;
use rand::{RngCore, thread_rng};
use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use rustix::fd::AsFd;
#[cfg(unix)]
use rustix::fs::{AtFlags, Mode, OFlags, openat, renameat, symlinkat};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn rand_suffix() -> String {
    let mut b = [0u8; 6];
    thread_rng().fill_bytes(&mut b);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b)
}

/// Fsync d’un répertoire (crucial pour la durabilité du rename et des entrées)
fn fsync_dir(dir: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use rustix::fs::{Advice, Mode as RMode, OFlags as ROFlags, open};
        let f = open(dir, ROFlags::RDONLY | ROFlags::DIRECTORY, RMode::empty())
            .with_context(|| format!("open(dir) for fsync: {}", dir.display()))?;
        rustix::fs::datasync(&f).context("fsync(dir)")?;
        // optionnel: f.advise(Advice::Normal).ok();
        Ok(())
    }
    #[cfg(windows)]
    {
        // Sur Windows, fsync de répertoire non-supporté; on fait au mieux côté fichiers.
        let _ = dir;
        Ok(())
    }
}

/// Copie « sûre » : flux + fsync du fichier destination
fn copy_file_fsync(src: &Path, dst: &Path) -> Result<()> {
    let mut in_f = fs::File::open(src).with_context(|| format!("open(src): {}", src.display()))?;
    let mut out_f =
        fs::File::create(dst).with_context(|| format!("create(dst): {}", dst.display()))?;
    std::io::copy(&mut in_f, &mut out_f).context("copy stream")?;
    out_f.sync_all().context("fsync(dst)")?;
    Ok(())
}

/// Détermine si l’erreur de rename est un cross-device (EXDEV)
fn is_exdev(err: &std::io::Error) -> bool {
    match err.raw_os_error() {
        #[cfg(unix)]
        Some(n) => n == libc::EXDEV,
        #[cfg(windows)]
        Some(_n) => false, // Windows renvoie d’autres codes; on tombera sur le fallback par échec explicite
        None => false,
    }
}

/// Déplace `src` dans `dst_dir` en générant un nom unique basé sur `basename`
/// Retourne le chemin final effectivement créé.
pub fn safe_move_unique(src: &Path, dst_dir: &Path, basename: &OsString) -> Result<PathBuf> {
    fs::create_dir_all(dst_dir).with_context(|| format!("create_dir_all {}", dst_dir.display()))?;
    fsync_dir(dst_dir).ok(); // best effort

    // Nom unique: <timestamp>__<rand>__<basename>
    let ts = chrono::Local::now().format("%Y%m%dT%H%M%S");
    let unique = format!(
        "{}__{}__{}",
        ts,
        rand_suffix(),
        Path::new(basename).to_string_lossy()
    );
    let dst = dst_dir.join(unique);

    // Essayons rename (atomique, même FS)
    match fs::rename(src, &dst) {
        Ok(()) => {
            // fsync du répertoire destination pour persister l’entrée
            fsync_dir(dst_dir)?;
            Ok(dst)
        }
        Err(e) if is_exdev(&e) => {
            // Cross-FS: copy + fsync + swap + unlink
            let tmp = dst.with_extension("copying");
            copy_recursively(src, &tmp)?;
            fsync_dir(tmp.parent().unwrap_or(dst_dir))?;
            fs::rename(&tmp, &dst).context("swap tmp->dst")?;
            fsync_dir(dst_dir)?;
            // Supprimer la source seulement après succès
            remove_recursively(src)?;
            Ok(dst)
        }
        Err(e) => Err(e).with_context(|| format!("rename {} -> {}", src.display(), dst.display())),
    }
}

/// Copie récursive minimaliste : fichiers/dirs/symlinks.
/// (Améliorable: ACL, xattrs…)
pub fn copy_recursively(src: &Path, dst: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(src)?;
    if meta.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let name = entry.file_name();
            copy_recursively(&entry.path(), &dst.join(name))?;
        }
        Ok(())
    } else if meta.file_type().is_symlink() {
        #[cfg(unix)]
        {
            let target = fs::read_link(src)?;
            std::os::unix::fs::symlink(target, dst)?;
            Ok(())
        }
        #[cfg(windows)]
        {
            // Sur Windows, différencier symlink file/dir est nécessaire.
            // Pour un premier jet, on résout le lien et on copie la cible.
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
