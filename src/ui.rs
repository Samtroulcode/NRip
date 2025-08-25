use anyhow::{Context, Result};
use chrono::{Local, TimeZone};
use std::process::{Command, Stdio};

use crate::index::{Index, Kind};
use yansi::{Color, Paint};

fn human_when(ts: i64) -> String {
    // Date locale courte (pour fzf)
    let dt = Local
        .timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(|| Local.timestamp_opt(0, 0).single().unwrap());
    dt.format("%Y-%m-%d %H:%M").to_string()
}

fn kind_icon(k: Kind) -> &'static str {
    match k {
        Kind::File => "📄",
        Kind::Dir => "📁",
        Kind::Symlink => "🔗",
        Kind::Other => "❔",
    }
}

/// Lignes pour fzf (compactes):
/// IDX \t ICON \t DATE \t BASENAME \t ORIGINAL \t TRASHED(HIDDEN)
fn build_fzf_lines(idx: &Index) -> Vec<String> {
    idx.items
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let icon = kind_icon(e.kind);
            let base = e
                .original_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            // Champ 0 (index) ***NON COLORÉ*** pour le parse
            let icon_p = Paint::new(icon).fg(Color::Cyan).to_string();
            let date_p = Paint::new(human_when(e.deleted_at)).dim().to_string();
            let base_p = Paint::new(base).bold().to_string();
            format!(
                "{}\t{}\t{}\t{}\t{}\t{}",
                i,
                icon_p,
                date_p,
                base_p,
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
        .args(["--with-nth", "3,4,5"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());

    if preview {
        // Aperçu simple: liste le chemin TRASHED (colonne après "->")
        cmd.args([
            "--preview",
            r#"sh -c 'printf "%s\n" "$@" | awk -F"\t" "{print \$NF}" | xargs -r ls -ld --'"#,
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
