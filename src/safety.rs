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
                Forbid::Root => "denied: / is protected (cannot be overridden)",
                Forbid::Dot => "denied: '.' is not allowed",
                Forbid::DotDot => "denied: '..' is not allowed",
                Forbid::GraveyardItself => "denied: target is the graveyard itself",
                Forbid::InsideGraveyard => "denied: item is inside the graveyard",
                Forbid::IndexFile => "denied: target is index.json/.index.lock",
                Forbid::JournalFile => "denied: target is .journal",
                Forbid::Empty => "denied: empty path",
            };
            bail!(msg);
        }
    }
    Ok(())
}
