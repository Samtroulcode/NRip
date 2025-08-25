use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Forbid {
    Root,
    GraveyardItself,
    InsideGraveyard,
    Empty,
}

#[derive(Debug, Clone)]
pub struct SafetyCtx {
    pub graveyard: PathBuf,
    pub preserve_root: bool, // default: true
    pub force: bool,         // -f
}

pub fn classify_forbid(p: &Path, ctx: &SafetyCtx) -> Option<Forbid> {
    if p.as_os_str().is_empty() {
        return Some(Forbid::Empty);
    }

    // 1) root
    #[cfg(unix)]
    if ctx.preserve_root && p == Path::new("/") {
        return Some(Forbid::Root);
    }

    // 2) graveyard
    if p == ctx.graveyard {
        return Some(Forbid::GraveyardItself);
    }
    if p.starts_with(&ctx.graveyard) {
        return Some(Forbid::InsideGraveyard);
    }

    None
}

pub fn guard_path(p: &Path, ctx: &SafetyCtx) -> anyhow::Result<()> {
    if let Some(reason) = classify_forbid(p, ctx) {
        // on autorise de contourner certains cas avec --force, sauf Root si preserve_root=true
        let can_bypass = ctx.force && !matches!(reason, Forbid::Root);
        if !can_bypass {
            use anyhow::bail;
            let msg = match reason {
                Forbid::Root => "refusé: / est protégé (utilisez --no-preserve-root pour forcer)",
                Forbid::GraveyardItself => "refusé: cible = graveyard lui-même",
                Forbid::InsideGraveyard => "refusé: élément à l’intérieur du graveyard",
                Forbid::Empty => "refusé: chemin vide",
            };
            bail!(msg);
        }
    }
    Ok(())
}
