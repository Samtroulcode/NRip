use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use rand::Rng;

use crate::index;

pub fn short_id() -> String {
    const ALPH: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mut rng = rand::thread_rng();
    (0..7)
        .map(|_| {
            let i = rng.gen_range(0..ALPH.len());
            ALPH[i] as char
        })
        .collect()
}

fn graveyard_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("~/.local/share"));
    base.join("riptide").join("graveyard")
}

pub fn bury(paths: Vec<PathBuf>) -> anyhow::Result<()> {
    let gy = graveyard_dir();
    fs::create_dir_all(&gy)?;

    for path in paths {
        if !path.exists() {
            eprintln!("{} not found", path.display());
            continue;
        }
        let dest = gy.join(path.file_name().unwrap());
        fs::rename(&path, &dest)?;
        index::add_entry(&path, &dest)?;
        println!("moved {} -> {}", path.display(), dest.display());
    }
    Ok(())
}

pub fn list() -> anyhow::Result<()> {
    let entries = index::load_entries().unwrap_or_default();
    for e in entries {
        let id = e.id.as_deref().unwrap_or("-");
        let base = index::basename_of_original(&e);
        println!("{:7}  {}  {}", id, e.deleted_at, base);
    }
    Ok(())
}

/// `prune` sans cible = vider tout ; avec cible = supprimer les matches.
pub fn prune(target: Option<String>, dry_run: bool, yes: bool) -> anyhow::Result<()> {
    let mut entries = index::load_entries().unwrap_or_default();

    // Sélection (NE PAS déplacer `target`)
    let to_delete: Vec<index::Entry> = if let Some(ref q) = target {
        let q_lc = q.to_lowercase();
        let matches: Vec<index::Entry> = entries
            .iter()
            .cloned()
            .filter(|e| {
                let base = index::basename_of_original(e).to_lowercase();
                let id = e.id.as_deref().unwrap_or("").to_lowercase();
                base.contains(&q_lc) || id.starts_with(&q_lc)
            })
            .collect();

        if matches.is_empty() {
            println!("No graveyard entry matches '{}'.", q);
            return Ok(());
        }
        // Si plusieurs et pas -y, on affiche et on abandonne (autocomplétion fera le tri)
        if matches.len() > 1 && !yes {
            println!("Multiple matches (use TAB completion or add -y to prune all):");
            for m in &matches {
                let id = m.id.as_deref().unwrap_or("-");
                println!("  {:7}  {}", id, index::basename_of_original(m));
            }
            return Ok(());
        }
        matches
    } else {
        // prune total
        entries.clone()
    };

    let is_all = target.is_none(); // ✅ on peut l'utiliser, on n'a pas déplacé `target`

    // Bilan
    let mut count = 0usize;
    let mut total_bytes: u64 = 0;
    for e in &to_delete {
        if let Ok(meta) = std::fs::metadata(&e.stored_path) {
            total_bytes = total_bytes.saturating_add(meta.len());
        }
    }
    let mb = (total_bytes as f64) / (1024.0 * 1024.0);

    if is_all {
        println!(
            "About to remove ALL graveyard items: {} items (~{:.2} MiB)",
            to_delete.len(),
            mb
        );
        if dry_run {
            println!("--dry-run: nothing deleted.");
            return Ok(());
        }
        if !yes {
            print!("Type YES to confirm: ");
            std::io::stdout().flush()?;
            let mut buf = String::new();
            std::io::stdin().read_line(&mut buf)?;
            if buf.trim() != "YES" {
                println!("Aborted.");
                return Ok(());
            }
        }
    } else {
        println!(
            "About to remove {} item(s) (~{:.2} MiB).",
            to_delete.len(),
            mb
        );
        if dry_run {
            println!("--dry-run: nothing deleted.");
            return Ok(());
        }
        if !yes && to_delete.len() == 1 {
            print!("Confirm (y/N): ");
            std::io::stdout().flush()?;
            let mut buf = String::new();
            std::io::stdin().read_line(&mut buf)?;
            if buf.trim().to_lowercase() != "y" {
                println!("Aborted.");
                return Ok(());
            }
        }
    }

    // Suppression des fichiers/dirs
    for e in &to_delete {
        let p = std::path::Path::new(&e.stored_path);
        let res = if p.is_dir() {
            std::fs::remove_dir_all(p)
        } else {
            std::fs::remove_file(p).or_else(|err| {
                if err.kind() == std::io::ErrorKind::IsADirectory {
                    std::fs::remove_dir_all(p)
                } else {
                    Err(err)
                }
            })
        };
        match res {
            Ok(_) => count += 1,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => count += 1,
            Err(err) => eprintln!("warn: cannot remove {}: {}", p.display(), err),
        }
    }

    // Mise à jour de l’index
    if is_all {
        index::save_entries(&Vec::new())?;
        // Nettoyage best-effort du dossier (⚠ remplacer `.flatten()` par `filter_map(Result::ok)`)
        let gy = graveyard_dir();
        if let Ok(rd) = std::fs::read_dir(&gy) {
            for ent in rd.filter_map(|r| r.ok()) {
                let p = ent.path();
                let _ = if p.is_dir() {
                    std::fs::remove_dir_all(&p)
                } else {
                    std::fs::remove_file(&p)
                };
            }
        }
    } else {
        let delete_set: std::collections::HashSet<String> =
            to_delete.iter().map(|e| e.stored_path.clone()).collect();
        entries.retain(|e| !delete_set.contains(&e.stored_path));
        index::save_entries(&entries)?;
    }

    println!("Removed {} item(s).", count);
    Ok(())
}

/// Candidats pour l’auto-complétion de `prune` (basenames + IDs)
pub fn completion_candidates(prefix: Option<&str>) -> anyhow::Result<Vec<String>> {
    let entries = index::load_entries().unwrap_or_default();
    let mut out = Vec::with_capacity(entries.len() * 2);
    for e in entries {
        if let Some(id) = &e.id {
            out.push(id.clone());
        }
        out.push(index::basename_of_original(&e));
    }
    if let Some(p) = prefix {
        let p = p.to_lowercase();
        out.retain(|s| s.to_lowercase().contains(&p));
    }
    out.sort();
    out.dedup();
    Ok(out)
}
