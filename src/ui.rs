// src/ui.rs
use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use std::process::{Command, Stdio};

use crate::index::Index;

fn human_when(ts: i64) -> String {
    let dt = DateTime::<Utc>::from_naive_utc_and_offset(
        NaiveDateTime::from_timestamp_opt(ts, 0).unwrap_or_default(),
        Utc,
    );
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Lignes pour fzf: "IDX \t DATE \t ORIGINAL \t -> \t TRASHED"
fn build_fzf_lines(idx: &Index) -> Vec<String> {
    idx.items
        .iter()
        .enumerate()
        .map(|(i, e)| {
            format!(
                "{}\t{}\t{}\t->\t{}",
                i,
                human_when(e.deleted_at),
                e.original_path.display(),
                e.trashed_path.display()
            )
        })
        .collect()
}

/// Lance fzf (obligatoire). Retourne les indices sélectionnés (dans idx.items).
pub fn pick_entries_with_fzf(idx: &Index, preview: bool) -> Result<Vec<usize>> {
    let lines = build_fzf_lines(idx);
    if lines.is_empty() {
        return Ok(vec![]);
    }

    let mut cmd = Command::new("fzf");
    cmd.arg("--multi") // multi-sélection
        .arg("--height=40%")
        .arg("--layout=reverse")
        .arg("--border")
        .arg("--ansi")
        .arg("--print0") // sortie NUL-delimitée
        .args(["--delimiter", "\t"]) // champs = tab
        .args(["--with-nth", "2.."]) // on masque l'IDX à l'affichage
        .args(["--accept-nth", "1"]) // ... mais on NE SORT que l'IDX
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());

    if preview {
        // Tu pourras raffiner plus tard (bat, ls -l, file, etc.).
        // Ici, on affiche la colonne "TRASHED" avec un ls -l simple :
        cmd.args([
            "--preview",
            r#"sh -c 'printf "%s\n" "$@" | awk -F"\t" "{for (i=1;i<=NF;i++) if (\$i==\"->\") { print \$(i+1); exit }}" | xargs -r ls -ld --'"#,
            "--preview-window=right:60%",
        ]);
    }

    let mut child = cmd.spawn().context("fzf non trouvé (installez `fzf`)")?;

    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().context("ouverture stdin fzf")?;
        for line in &lines {
            writeln!(stdin, "{line}")?;
        }
    }

    let out = child.wait_with_output().context("exécution fzf")?;
    if !out.status.success() {
        // 1 = no match / 130 = ESC/Ctrl-C → on considère “aucune sélection”.
        return Ok(vec![]);
    }

    // Grâce à --accept-nth=1, la sortie contient UNIQUEMENT les indices, séparés par NUL.
    let mut selected = Vec::new();
    for part in out.stdout.split(|&b| b == 0u8).filter(|s| !s.is_empty()) {
        if let Ok(i) = std::str::from_utf8(part)
            .ok()
            .and_then(|s| s.trim().parse::<usize>().ok())
        {
            if i < idx.items.len() {
                selected.push(i);
            }
        }
    }
    selected.sort_unstable();
    selected.dedup();
    Ok(selected)
}
