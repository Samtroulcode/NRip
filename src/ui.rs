use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use std::process::{Command, Stdio};

fn human_when(ts: i64) -> String {
    let dt = DateTime::<Utc>::from_naive_utc_and_offset(
        NaiveDateTime::from_timestamp_opt(ts, 0).unwrap_or_default(),
        Utc,
    );
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Construit les lignes pour fzf: "IDX\tDATE\tORIGINAL -> TRASHED"
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

/// Lance fzf en multi-sélection et renvoie les indices sélectionnés dans `idx.items`
pub fn pick_entries_with_fzf(idx: &Index, preview: bool) -> Result<Vec<usize>> {
    let lines = build_fzf_lines(idx);
    if lines.is_empty() {
        return Ok(vec![]);
    }

    let mut cmd = Command::new("fzf");
    cmd.arg("-m")
        .arg("--height=40%")
        .arg("--layout=reverse")
        .arg("--border")
        .arg("--ansi")
        // on ne renvoie que la 1re colonne (l’index) à la sortie:
        .args(["--with-nth", "2.."])
        .args(["--print0"]) // sortie NUL-separated pour éviter les surprises
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());

    if preview {
        // Aperçu simple: `ls -l` de la cible dans le graveyard (col 4)
        // {n} = nth token (1-based) côté fzf; on a masqué la 1re col à l’affichage,
        // mais elle reste en entrée. Pour un preview plus riche, redonne l’index
        // en --preview-label et reparse; ici on fait simple: on reconstruit via awk.
        cmd.args([
            "--preview",
            r#"awk -F'\t' '{for (i=1;i<=NF;i++) if ($i=="->") { print $(i+1); exit }}' <<< {+}"#,
        ]);
    }

    let mut child = cmd.spawn().context("spawn fzf")?;

    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().context("open fzf stdin")?;
        for line in &lines {
            // On donne TOUTES les colonnes à fzf (index en 1er champ)
            writeln!(stdin, "{line}")?;
        }
    }

    let out = child.wait_with_output().context("wait fzf")?;
    if !out.status.success() {
        return Ok(vec![]); // échappé/annulé
    }

    // out.stdout contient les lignes sélectionnées, NUL-separated (print0)
    let raw = out.stdout;
    let parts = raw.split(|&b| b == 0u8).filter(|s| !s.is_empty());

    // Chaque partie est LA LIGNE ENTIÈRE sélectionnée (pas seulement la 1re col)
    // On relit l’index (1re col) pour mapper:
    let mut selected = Vec::new();
    for part in parts {
        let s = String::from_utf8_lossy(part);
        if let Some(first_field) = s.split('\t').next() {
            if let Ok(i) = first_field.parse::<usize>() {
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

/// Variante simple: détecte l’absence de fzf (NotFound) -> renvoie Ok(vec![]) pour fallback.
pub fn try_pick_entries(idx: &Index, preview: bool) -> Result<Vec<usize>> {
    match pick_entries_with_fzf(idx, preview) {
        Err(e)
            if e.root_cause()
                .to_string()
                .contains("No such file or directory")
                || e.root_cause().to_string().contains("not found") =>
        {
            // fzf non installé
            Ok(vec![])
        }
        other => other,
    }
}
