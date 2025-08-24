use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::index::Entry;
use std::collections::HashSet;

use anyhow::{Context, Result};
use chrono::Utc;
use fs_err as fs;
use std::ffi::OsString;

use crate::fs_safemove::safe_move_unique;
use crate::index::{load_index, save_index};

use crate::index; // pour appeler les shims

fn display_id(e: &index::Entry) -> String {
    e.trashed_path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|s| s.split("__").nth(1))
        .map(|s| s.chars().take(7).collect::<String>())
        .unwrap_or_else(|| "-".to_string())
}

fn graveyard_dir() -> Result<PathBuf> {
    let data = crate::paths::data_dir()?;
    let gy = data.join("nrip").join("graveyard");
    fs::create_dir_all(&gy)?;
    Ok(gy)
}

fn journal_path() -> Result<PathBuf> {
    Ok(graveyard_dir()?.join(".journal"))
}

fn append_journal(line: &str) -> Result<()> {
    use std::io::Write;
    let jp = journal_path()?;
    let mut f = fs::OpenOptions::new().create(true).append(true).open(&jp)?;
    writeln!(f, "{}", line)?;
    f.sync_all()?;
    Ok(())
}

pub fn resurrect(items: &[PathBuf]) -> Result<()> {
    // items : chemins dans la graveyard (ou bien indices depuis l’index)
    let mut idx = load_index()?;
    for gy_path in items {
        // Retrouver original dans l’index
        if let Some(pos) = idx.items.iter().position(|e| e.trashed_path == *gy_path) {
            let original = idx.items[pos].original_path.clone();

            append_journal(&format!(
                "RESTORE_PENDING\t{}\t{}",
                gy_path.display(),
                original.display()
            ))?;
            // Créer le dossier parent si nécessaire (restauration)
            if let Some(parent) = original.parent() {
                fs::create_dir_all(parent)?;
            }
            // move inverse (unique_name pas nécessaire ici → on veut retrouver le nom exact)
            // mais s'il existe déjà, on refuse (éviter écrasement) :
            if original.exists() {
                anyhow::bail!("La cible existe déjà: {}", original.display());
            }

            // Implémentation simple : essayer rename, sinon fallback copy+unlink
            match fs::rename(gy_path, &original) {
                Ok(()) => {}
                Err(e) if super::fs_safemove::is_exdev(&e) => {
                    super::fs_safemove::copy_recursively(gy_path, &original)?;
                    super::fs_safemove::remove_recursively(gy_path)?;
                }
                Err(e) => {
                    return Err(e).with_context(|| {
                        format!("rename {} -> {}", gy_path.display(), original.display())
                    });
                }
            }

            append_journal(&format!(
                "RESTORE_DONE\t{}\t{}",
                gy_path.display(),
                original.display()
            ))?;
            idx.items.remove(pos);
        } else {
            // si pas d'entrée, on tente quand même la restauration best‑effort
            // (utile après crash si l’index n’a pas été sync).
            // Ici, on pourrait logguer/ignorer selon le choix de UX.
        }
    }
    save_index(&idx)?;
    Ok(())
}

pub fn bury(paths: &[PathBuf]) -> Result<()> {
    let gy = graveyard_dir()?;
    let mut idx = load_index()?;

    for src in paths {
        let base: OsString = src.file_name().unwrap_or_default().to_os_string();
        append_journal(&format!(
            "PENDING\t{}\t{}",
            src.display(),
            base.to_string_lossy()
        ))?;
        let dst = safe_move_unique(src, &gy, &base)
            .with_context(|| format!("move {} -> graveyard", src.display()))?;
        append_journal(&format!("DONE\t{}\t{}", src.display(), dst.display()))?;

        idx.items.push(Entry {
            original_path: src.clone(),
            trashed_path: dst.clone(),
            deleted_at: Utc::now().timestamp(),
        });
    }
    save_index(&idx)?;
    Ok(())
}

pub fn list() -> anyhow::Result<()> {
    let entries = index::load_entries().unwrap_or_default();
    for e in entries {
        let id = display_id(&e);
        let base = index::basename_of_original(&e);
        println!(
            "{:7}  {}  {} ({})",
            id,
            e.deleted_at,
            base,
            e.original_path.display()
        );
    }
    Ok(())
}

/// `prune` sans cible = vider tout ; avec cible = supprimer les matches.
pub fn prune(target: Option<String>, dry_run: bool, yes: bool) -> anyhow::Result<()> {
    let mut entries = index::load_entries().unwrap_or_default();

    // 1) Construire la liste à supprimer (to_delete)
    let to_delete: Vec<index::Entry> = if let Some(ref q0) = target {
        let q = q0.to_lowercase();

        let matches: Vec<index::Entry> = entries
            .iter()
            .cloned()
            .filter(|e| {
                let base = index::basename_of_original(e).to_lowercase();
                let id = display_id(e).to_lowercase();
                base.contains(&q) || id.starts_with(&q)
            })
            .collect();

        if matches.is_empty() {
            println!("No graveyard entry matches '{}'.", q0);
            return Ok(());
        }
        if matches.len() > 1 && !yes {
            println!("Multiple matches (use TAB completion or add -y to prune all of them):");
            for m in &matches {
                let id = display_id(m);
                println!("  {:7}  {}", id, index::basename_of_original(m));
            }
            return Ok(());
        }
        matches
    } else {
        // --- MODE INTERACTIF ---
        if entries.is_empty() {
            println!("Graveyard is empty.");
            return Ok(());
        }
        println!("Select an item to delete or choose 0) ALL:");
        println!("  0) ALL");
        for (i, e) in entries.iter().enumerate() {
            let id = display_id(e);
            let base = index::basename_of_original(e);
            println!(
                "{:3}) {:7}  {}  ({})",
                i + 1,
                id,
                base,
                e.original_path.display()
            );
        }
        print!("Choice [0=ALL, q=cancel]: ");
        io::stdout().flush()?;
        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        let s = buf.trim().to_lowercase();
        if s == "q" || s.is_empty() {
            println!("Aborted.");
            return Ok(());
        }
        if s == "0" {
            entries.clone() // ALL
        } else {
            let sel: usize = s.parse().unwrap_or(usize::MAX);
            if sel == 0 || sel > entries.len() {
                println!("Invalid choice.");
                return Ok(());
            }
            vec![entries[sel - 1].clone()]
        }
    };

    // 2) Bilan et confirmations
    let mut total_bytes: u64 = 0;
    for e in &to_delete {
        if let Ok(meta) = fs::metadata(&e.trashed_path) {
            total_bytes = total_bytes.saturating_add(meta.len());
        }
    }
    let mb = (total_bytes as f64) / (1024.0 * 1024.0);

    let is_all = to_delete.len() == entries.len();
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
            io::stdout().flush()?;
            let mut buf = String::new();
            io::stdin().read_line(&mut buf)?;
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
            io::stdout().flush()?;
            let mut buf = String::new();
            io::stdin().read_line(&mut buf)?;
            if buf.trim().to_lowercase() != "y" {
                println!("Aborted.");
                return Ok(());
            }
        }
    }

    // 3) Suppression des fichiers/dirs
    let mut removed = 0usize;
    for e in &to_delete {
        let p: &Path = &e.trashed_path;
        let res = if p.is_dir() {
            fs::remove_dir_all(p)
        } else {
            fs::remove_file(p).or_else(|err| {
                if err.kind() == std::io::ErrorKind::IsADirectory {
                    fs::remove_dir_all(p)
                } else {
                    Err(err)
                }
            })
        };
        match res {
            Ok(_) => removed += 1,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => removed += 1,
            Err(err) => eprintln!("warn: cannot remove {}: {}", p.display(), err),
        }
    }

    // 4) Mise à jour de l’index + nettoyage dossier
    if is_all {
        index::save_entries(&Vec::new())?;
        let gy = graveyard_dir()?; // <— ta version renvoie Result<PathBuf>
        if let Ok(rd) = fs::read_dir(&gy) {
            for ent in rd.filter_map(|r| r.ok()) {
                let p = ent.path();
                let _ = if p.is_dir() {
                    fs::remove_dir_all(&p)
                } else {
                    fs::remove_file(&p)
                };
            }
        }
    } else {
        let delete_set: HashSet<PathBuf> =
            to_delete.iter().map(|e| e.trashed_path.clone()).collect();
        entries.retain(|e| !delete_set.contains(&e.trashed_path));
        index::save_entries(&entries)?;
    }

    println!("Removed {} item(s).", removed);
    Ok(())
}

/// Candidats pour l’auto-complétion de `prune` (basenames + IDs)
pub fn completion_candidates(prefix: Option<&str>) -> anyhow::Result<Vec<String>> {
    let entries = index::load_entries().unwrap_or_default();
    let mut out = Vec::with_capacity(entries.len() * 2);
    for e in entries {
        out.push(display_id(&e)); // <- id dérivé (7 chars)
        out.push(index::basename_of_original(&e)); // <- basename
    }
    if let Some(p) = prefix {
        let p = p.to_lowercase();
        out.retain(|s| s.to_lowercase().contains(&p));
    }
    out.sort();
    out.dedup();
    Ok(out)
}
