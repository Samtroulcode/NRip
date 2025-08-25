use anyhow::{Context, Result};
use chrono::{TimeZone, Utc};
use std::process::{Command, Stdio};

use crate::index::Index;

fn human_when(ts: i64) -> String {
    let dt = Utc
        .timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(|| Utc.timestamp_opt(0, 0).single().unwrap());
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
        .args(["--with-nth", "2.."]) // masque l'IDX à l'affichage
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());

    if preview {
        // Aperçu simple: liste le chemin TRASHED (colonne après "->")
        cmd.args([
            "--preview",
            r#"sh -c 'printf "%s\n" "$@" | awk -F"\t" "{for (i=1;i<=NF;i++) if (\$i==\"->\") { print \$(i+1); exit }}" | xargs -r ls -ld --'"#,
            "--preview-window=right:60%",
        ]);
    }

    let mut child = cmd
        .spawn()
        .context("fzf not found (please install `fzf`)")?;

    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().context("open fzf stdin")?;
        for line in &lines {
            writeln!(stdin, "{line}")?;
        }
    }

    let out = child.wait_with_output().context("run fzf")?;
    if !out.status.success() {
        return Ok(vec![]);
    }

    let mut selected = Vec::new();
    for part in out.stdout.split(|&b| b == 0u8).filter(|s| !s.is_empty()) {
        let s = match std::str::from_utf8(part) {
            Ok(x) => x,
            Err(_) => continue,
        };
        if let Some(first_field) = s.split('\t').next() {
            if let Ok(i) = first_field.trim().parse::<usize>() {
                if i < idx.items.len() {
                    selected.push(i);
                }
            }
        }
    }
    selected.sort_unstable();
    selected.dedup();
    Ok(selected)
}
