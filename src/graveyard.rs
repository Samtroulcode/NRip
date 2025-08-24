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

// ---------- helpers copie/rename ----------
fn copy_file(src: &Path, dst: &Path) -> anyhow::Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(src, dst)?;
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else if ty.is_file() {
            copy_file(&src_path, &dst_path)?;
        } else if ty.is_symlink() {
            let target = fs::read_link(&src_path)?;
            std::os::unix::fs::symlink(target, &dst_path)?;
        }
    }
    Ok(())
}

fn move_across_fs(src: &Path, dst: &Path) -> anyhow::Result<()> {
    if src.is_dir() {
        copy_dir_all(src, dst)?;
        fs::remove_dir_all(src)?;
    } else {
        copy_file(src, dst)?;
        fs::remove_file(src)?;
    }
    Ok(())
}

fn safe_move(src: &Path, dst: &Path) -> anyhow::Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::rename(src, dst) {
        Ok(_) => Ok(()),
        Err(e) => {
            // EXDEV (cross-device) -> copie + unlink
            if e.raw_os_error() == Some(18) {
                move_across_fs(src, dst)
            } else {
                // tente tout de même la voie "copie" s’il y a un autre souci d’atomicité
                move_across_fs(src, dst)
            }
        }
    }
}
// ------------------------------------------

pub fn resurrect(target: Option<String>, yes: bool) -> anyhow::Result<()> {
    let mut entries = index::load_entries().unwrap_or_default();

    // sélection
    let chosen: index::Entry = if let Some(q) = target {
        let q = q.to_lowercase();
        let mut matches: Vec<index::Entry> = entries
            .iter()
            .cloned()
            .filter(|e| {
                let base = index::basename_of_original(e).to_lowercase();
                let id = e.id.as_deref().unwrap_or("").to_lowercase();
                base.contains(&q) || id.starts_with(&q)
            })
            .collect();

        if matches.is_empty() {
            println!("No graveyard entry matches '{}'.", q);
            return Ok(());
        }
        if matches.len() > 1 && !yes {
            println!("Multiple matches. Be more specific or pick one:");
            for (i, m) in matches.iter().enumerate() {
                let id = m.id.as_deref().unwrap_or("-");
                println!("{:2}) {:7}  {}", i + 1, id, m.original_path);
            }
            return Ok(());
        }
        matches.remove(0)
    } else {
        // prompt interactif
        if entries.is_empty() {
            println!("Graveyard is empty.");
            return Ok(());
        }
        println!("Please select a file/folder or item to resurrect:");
        for (i, e) in entries.iter().enumerate() {
            let id = e.id.as_deref().unwrap_or("-");
            let base = index::basename_of_original(e);
            println!("{:2}) {:7}  {}  ({})", i + 1, id, base, e.original_path);
        }
        print!("Enter number (0 to cancel): ");
        io::stdout().flush()?;
        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        let sel: usize = buf.trim().parse().unwrap_or(0);
        if sel == 0 || sel > entries.len() {
            println!("Aborted.");
            return Ok(());
        }
        entries[sel - 1].clone()
    };

    let src = PathBuf::from(&chosen.stored_path);
    let mut dst = PathBuf::from(&chosen.original_path);

    if !src.exists() {
        eprintln!("Stored item not found: {}", src.display());
        // Purge de l’entrée orpheline
        entries.retain(|e| e.stored_path != chosen.stored_path);
        index::save_entries(&entries)?;
        return Ok(());
    }

    // collision à destination
    if dst.exists() {
        if !yes {
            print!("Destination exists: {}. Overwrite? [y/N]: ", dst.display());
            io::stdout().flush()?;
            let mut ans = String::new();
            io::stdin().read_line(&mut ans)?;
            if ans.trim().to_lowercase() != "y" {
                // suffixe .restored (ou avec id)
                let base = dst
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "restored".into());
                let mut alt = dst.clone();
                let id = chosen.id.as_deref().unwrap_or("restored");
                let suffixed = format!("{}.{}.restored", base, id);
                alt.set_file_name(suffixed);
                println!("Restoring to alternative path: {}", alt.display());
                dst = alt;
            }
        }
    }

    // move (rename ou copie+unlink)
    safe_move(&src, &dst)?;

    // MAJ index: retire l’entrée
    entries.retain(|e| e.stored_path != chosen.stored_path);
    index::save_entries(&entries)?;

    println!("Resurrected -> {}", dst.display());
    Ok(())
}

pub fn bury(paths: Vec<PathBuf>) -> anyhow::Result<()> {
    let gy = graveyard_dir();
    fs::create_dir_all(&gy)?;

    for path in paths {
        if !path.exists() {
            eprintln!("{} not found", path.display());
            continue;
        }

        let original_abs: PathBuf = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // fallback si canonicalize échoue (ex: permissions) : CWD + path
                std::env::current_dir()?.join(&path)
            }
        };

        // destination (tu gardes ton schéma actuel)
        let dest = gy.join(path.file_name().unwrap());

        // déplacer (rename ou copie inter-FS via safe_move si tu préfères)
        fs::rename(&path, &dest)?;

        // ✅ On enregistre l'ABSOLU dans l'index
        index::add_entry(&original_abs, &dest)?;

        println!("moved {} -> {}", original_abs.display(), dest.display());
    }
    Ok(())
}

pub fn list() -> anyhow::Result<()> {
    let entries = index::load_entries().unwrap_or_default();
    for e in entries {
        let id = e.id.as_deref().unwrap_or("-");
        let base = index::basename_of_original(&e);
        println!("{:7}  {}  {} ({})", id, e.deleted_at, base, e.original_path);
    }
    Ok(())
}

/// `prune` sans cible = vider tout ; avec cible = supprimer les matches.
pub fn prune(target: Option<String>, dry_run: bool, yes: bool) -> anyhow::Result<()> {
    let mut entries = index::load_entries().unwrap_or_default();

    // 1) Construire la liste à supprimer (to_delete)
    let to_delete: Vec<index::Entry> = if let Some(ref q0) = target {
        let q = q0.to_lowercase();
        let mut matches: Vec<index::Entry> = entries
            .iter()
            .cloned()
            .filter(|e| {
                let base = index::basename_of_original(e).to_lowercase();
                let id = e.id.as_deref().unwrap_or("").to_lowercase();
                base.contains(&q) || id.starts_with(&q)
            })
            .collect();

        if matches.is_empty() {
            println!("No graveyard entry matches '{}'.", q0);
            return Ok(());
        }
        // Plusieurs résultats sans -y : afficher et sortir (l'auto-complétion aidera)
        if matches.len() > 1 && !yes {
            println!("Multiple matches (use TAB completion or add -y to prune all of them):");
            for m in &matches {
                let id = m.id.as_deref().unwrap_or("-");
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
            let id = e.id.as_deref().unwrap_or("-");
            let base = index::basename_of_original(e);
            println!("{:3}) {:7}  {}  ({})", i + 1, id, base, e.original_path);
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
        if let Ok(meta) = fs::metadata(&e.stored_path) {
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
        let p = Path::new(&e.stored_path);
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
        let gy = graveyard_dir();
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
        let delete_set: std::collections::HashSet<String> =
            to_delete.iter().map(|e| e.stored_path.clone()).collect();
        entries.retain(|e| !delete_set.contains(&e.stored_path));
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
