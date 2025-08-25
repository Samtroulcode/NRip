use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::path;
use std::path::PathBuf;

use crate::index::Entry;

use anyhow::{Context, Result};
use chrono::Utc;
use fs_err as fs;
use std::ffi::OsString;

use crate::fs_safemove::safe_move_unique;
use crate::safety::{SafetyCtx, guard_path};

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
    Ok(crate::paths::data_dir()?.join("graveyard"))
}

fn journal_path() -> Result<PathBuf> {
    Ok(graveyard_dir()?.join(".journal"))
}

fn append_journal(line: &str) -> Result<()> {
    use std::io::Write;
    let jp = journal_path()?;
    let mut f = fs::OpenOptions::new().create(true).append(true).open(&jp)?;
    writeln!(f, "{line}")?;
    f.sync_all()?;
    Ok(())
}

fn path_depth(p: &std::path::Path) -> usize {
    p.components().count()
}

///build a map from original_path -> (index position)
fn build_original_map(entries: &[index::Entry]) -> HashMap<PathBuf, usize> {
    let mut map = HashMap::with_capacity(entries.len());
    for (i, e) in entries.iter().enumerate() {
        map.insert(e.original_path.clone(), i);
    }
    map
}

pub fn resurrect(items: &[PathBuf]) -> Result<()> {
    index::with_index_mut(|idx| {
        for gy_path in items {
            if let Some(pos) = idx.items.iter().position(|e| e.trashed_path == *gy_path) {
                let original = idx.items[pos].original_path.clone();

                append_journal(&format!(
                    "RESTORE_PENDING\t{}\t{}",
                    gy_path.display(),
                    original.display()
                ))?;

                if let Some(parent) = original.parent() {
                    fs::create_dir_all(parent)?;
                }
                if original.exists() {
                    anyhow::bail!("Target already exists: {}", original.display());
                }

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
            }
        }
        Ok(())
    })
}

pub fn resurrect_cmd(target: Option<String>, dry_run: bool, yes: bool) -> anyhow::Result<()> {
    use std::io::{self, Write};

    let entries = index::load_entries().unwrap_or_default();
    let original_map = build_original_map(&entries);

    // 1) Construire la sélection (to_restore)
    let to_restore: Vec<index::Entry> = if let Some(ref q0) = target {
        let q = q0.to_lowercase();

        let matches: Vec<index::Entry> = entries
            .iter()
            .filter(|&e| {
                let base = index::basename_of_original(e).to_lowercase();
                let id = display_id(e).to_lowercase(); // derived id
                base.contains(&q) || id.starts_with(&q)
            })
            .cloned()
            .collect();

        if matches.is_empty() {
            println!("No graveyard entry matches '{q0}'.");
            return Ok(());
        }
        if matches.len() > 1 && !yes {
            println!("Multiple matches (use TAB completion or add -y to restore all of them):");
            for m in &matches {
                let id = display_id(m);
                println!("  {:7}  {}", id, index::basename_of_original(m));
            }
            return Ok(());
        }
        matches
    } else {
        // --- MODE INTERACTIF (fzf) ---
        let idx = index::load_index()?;
        if idx.items.is_empty() {
            println!("Graveyard is empty.");
            return Ok(());
        }

        let picks = crate::ui::pick_entries_with_fzf(&idx, /*preview=*/ false)?;
        if picks.is_empty() {
            println!("Aborted.");
            return Ok(());
        }

        let to_restore: Vec<index::Entry> =
            picks.into_iter().map(|i| idx.items[i].clone()).collect();

        // on “remplace” la variable to_restore de ton flux actuel :
        to_restore
    };
    // 1.b) Etendre la sélection : ajouter les parents enterrés nécessaires
    // (si un parent est lui-même dans le graveyard, on le restaure AVANT l'enfant)
    let mut wanted: HashSet<PathBuf> = to_restore.iter().map(|e| e.original_path.clone()).collect();
    let mut added_any = true;
    while added_any {
        added_any = false;
        let current: Vec<PathBuf> = wanted.iter().cloned().collect();
        for p in current {
            if let Some(mut cur) = p.parent().map(|x| x.to_path_buf()) {
                while cur.parent().is_some() {
                    if original_map.contains_key(&cur) && !wanted.contains(&cur) {
                        wanted.insert(cur.clone());
                        added_any = true;
                        break; // on insère ce parent; on regardera ses parents à l'itération suivante
                    }
                    if let Some(next) = cur.parent() {
                        cur = next.to_path_buf();
                    } else {
                        break;
                    }
                }
            }
        }
    }

    // Reconstruire la liste finale d'entrées à restaurer (parents inclus)
    let mut final_list: Vec<index::Entry> = Vec::with_capacity(wanted.len());
    let mut auto_added: Vec<PathBuf> = Vec::new();
    for p in &wanted {
        if let Some(&i) = original_map.get(p) {
            // si pas déjà dans to_restore explicite, on note qu'on l'a ajouté
            if !to_restore.iter().any(|e| &e.original_path == p) {
                auto_added.push(p.clone());
            }
            final_list.push(entries[i].clone());
        }
    }
    // Trier par profondeur croissante (parents → enfants)
    final_list.sort_by_key(|e| path_depth(&e.original_path));

    if !auto_added.is_empty() {
        println!(
            "Including {} parent path(s) for consistency:",
            auto_added.len()
        );
        for p in auto_added.iter().take(10) {
            // évite le spam
            println!("  {}", p.display());
        }
        if auto_added.len() > 10 {
            println!("  ...");
        }
    }

    // 2) Bilan & confirmations
    let is_all = final_list.len() == entries.len();
    if is_all {
        println!(
            "About to restore ALL graveyard items: {} item(s).",
            final_list.len()
        );
        if dry_run {
            println!("--dry-run: nothing restored.");
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
        println!("About to restore {} item(s).", to_restore.len());
        if dry_run {
            println!("--dry-run: nothing restored.");
            return Ok(());
        }
        if !yes && to_restore.len() == 1 {
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

    // 3) Exécution
    let paths: Vec<PathBuf> = final_list.iter().map(|e| e.trashed_path.clone()).collect();
    if paths.is_empty() {
        println!("Nothing to restore.");
        return Ok(());
    }

    // On réutilise ta fonction existante (journal, checks, msg "Restored to ...")
    resurrect(&paths)?;

    println!("Restored {} item(s).", paths.len());
    Ok(())
}

pub fn bury(paths: &[PathBuf], force: bool) -> Result<()> {
    let gy = graveyard_dir()?;
    let ctx = SafetyCtx {
        graveyard: gy.clone(),
        preserve_root: true,
        force,
    };

    index::with_index_mut(|idx| {
        for src in paths {
            let original_abs =
                path::absolute(src).with_context(|| format!("absolutize {}", src.display()))?;
            guard_path(&original_abs, &ctx)?;
            let base: OsString = src.file_name().unwrap_or_default().to_os_string();

            append_journal(&format!(
                "PENDING\t{}\t{}",
                original_abs.display(),
                base.to_string_lossy()
            ))?;
            let dst = safe_move_unique(src, &gy, &base)
                .with_context(|| format!("move {} -> graveyard", src.display()))?;
            append_journal(&format!(
                "DONE\t{}\t{}",
                original_abs.display(),
                dst.display()
            ))?;

            idx.items.push(Entry {
                original_path: original_abs,
                trashed_path: dst,
                deleted_at: Utc::now().timestamp(),
            });
        }
        Ok(())
    })
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
    // --- 1) SNAPSHOT & SÉLECTION (hors verrou) ---
    let snap = index::load_index()?; // snapshot
    if snap.items.is_empty() {
        println!("Graveyard is empty.");
        return Ok(());
    }

    // Construire la sélection "to_delete" depuis le snapshot
    let to_delete: Vec<index::Entry> = if let Some(ref q0) = target {
        let q = q0.to_lowercase();
        let matches: Vec<index::Entry> = snap
            .items
            .iter()
            .filter(|&e| {
                let base = index::basename_of_original(e).to_lowercase();
                let id = display_id(e).to_lowercase();
                base.contains(&q) || id.starts_with(&q)
            })
            .cloned()
            .collect();
        if matches.is_empty() {
            println!("No graveyard entry matches '{q0}'.");
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
        // Interactif (fzf) sur le snapshot
        let picks = crate::ui::pick_entries_with_fzf(&snap, /*preview=*/ false)?;
        if picks.is_empty() {
            println!("Aborted.");
            return Ok(());
        }
        picks.into_iter().map(|i| snap.items[i].clone()).collect()
    };

    // Rien ?
    if to_delete.is_empty() {
        println!("Nothing to delete.");
        return Ok(());
    }

    // Bilan (hors verrou)
    let mut total_bytes: u64 = 0;
    for e in &to_delete {
        if let Ok(meta) = fs::metadata(&e.trashed_path) {
            total_bytes = total_bytes.saturating_add(meta.len());
        }
    }
    let mb = (total_bytes as f64) / (1024.0 * 1024.0);

    let is_all = to_delete.len() == snap.items.len();
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

    // --- 2) COMMIT ATOMIQUE (sous verrou unique) ---
    let set: HashSet<PathBuf> = to_delete.iter().map(|e| e.trashed_path.clone()).collect();

    let removed = index::with_index_mut(|idx| {
        // Revalide la sélection côté index courant (au cas où ça a bougé)
        let mut remaining: Vec<Entry> = Vec::with_capacity(idx.items.len());
        let mut removed_count = 0usize;

        for e in idx.items.drain(..) {
            if set.contains(&e.trashed_path) {
                // Supprimer la cible (fichier/dir)
                let p = &e.trashed_path;
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
                    Ok(_) => removed_count += 1,
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => removed_count += 1,
                    Err(err) => {
                        eprintln!("warn: cannot remove {}: {}", p.display(), err);
                        // Échec : on conserve l’entrée
                        remaining.push(e);
                        continue;
                    }
                }
                // Succès → on ne remet pas l'entrée (deleted)
            } else {
                remaining.push(e);
            }
        }

        // Si la sélection couvrait "tout", on peut en plus nettoyer les résidus
        // du graveyard sans toucher aux méta (.journal/.index.lock)
        if is_all {
            let gy = crate::paths::data_dir()?.join("graveyard");
            if let Ok(rd) = fs::read_dir(&gy) {
                for ent in rd.filter_map(|r| r.ok()) {
                    let p = ent.path();
                    let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    if name == ".journal" || name == ".index.lock" {
                        continue;
                    }
                    let _ = if p.is_dir() {
                        fs::remove_dir_all(&p)
                    } else {
                        fs::remove_file(&p)
                    };
                }
            }
            remaining.clear(); // au cas où
        }

        idx.items = remaining;
        Ok(removed_count)
    })?;

    println!("Removed {removed} item(s).");
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
