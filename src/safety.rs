use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Forbid {
    Root,
    Dot,
    DotDot,
    GraveyardItself,
    InsideGraveyard,
    IndexFile,
    JournalFile,
    Empty,
}

#[derive(Debug, Clone)]
pub struct SafetyCtx {
    pub graveyard: PathBuf,
    pub preserve_root: bool,
    pub force: bool,
}

fn is_index_like(p: &Path) -> bool {
    if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
        name == "index.json" || name == ".index.lock"
    } else {
        false
    }
}

fn is_journal(p: &Path) -> bool {
    p.file_name().and_then(|s| s.to_str()) == Some(".journal")
}

pub fn classify_forbid(p: &Path, ctx: &SafetyCtx) -> Option<Forbid> {
    if p.as_os_str().is_empty() {
        return Some(Forbid::Empty);
    }
    if p == Path::new(".") {
        return Some(Forbid::Dot);
    }
    if p == Path::new("..") {
        return Some(Forbid::DotDot);
    }
    #[cfg(unix)]
    if ctx.preserve_root && p == Path::new("/") {
        return Some(Forbid::Root);
    }
    if p == ctx.graveyard {
        return Some(Forbid::GraveyardItself);
    }
    if p.starts_with(&ctx.graveyard) {
        return Some(Forbid::InsideGraveyard);
    }
    if is_index_like(p) {
        return Some(Forbid::IndexFile);
    }
    if is_journal(p) {
        return Some(Forbid::JournalFile);
    }
    None
}

pub fn guard_path(p: &Path, ctx: &SafetyCtx) -> anyhow::Result<()> {
    if let Some(reason) = classify_forbid(p, ctx) {
        let can_bypass = ctx.force && !matches!(reason, Forbid::Root);
        if !can_bypass {
            use anyhow::bail;
            let msg = match reason {
                Forbid::Root => "refusé: / est protégé (non contournable)",
                Forbid::Dot => "refusé: '.' n'est pas autorisé",
                Forbid::DotDot => "refusé: '..' n'est pas autorisé",
                Forbid::GraveyardItself => "refusé: cible = graveyard lui-même",
                Forbid::InsideGraveyard => "refusé: élément à l’intérieur du graveyard",
                Forbid::IndexFile => "refusé: cible index.json/.index.lock",
                Forbid::JournalFile => "refusé: cible .journal",
                Forbid::Empty => "refusé: chemin vide",
            };
            bail!(msg);
        }
    }
    Ok(())
}
