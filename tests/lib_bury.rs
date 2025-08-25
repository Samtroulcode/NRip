use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use serial_test::serial;
use std::process::Command;
mod util;

#[test]
#[serial]
fn bury_twice_same_basename_produces_distinct_targets() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;

    // Isole l'env XDG/HOME (base, sans /nrip)
    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    // Prépare deux fichiers distincts ayant le même nom ("dup.txt") dans 2 dossiers
    let d1 = tmp.child("a");
    d1.create_dir_all()?;
    let d2 = tmp.child("b");
    d2.create_dir_all()?;

    let f1 = d1.child("dup.txt");
    f1.write_str("A")?;
    let f2 = d2.child("dup.txt");
    f2.write_str("B")?;

    // enterre f1 puis f2
    Command::cargo_bin("nrip")?
        .arg(f1.path())
        .assert()
        .success();
    Command::cargo_bin("nrip")?
        .arg(f2.path())
        .assert()
        .success();

    // Lis l'index et récupère les "trashed_path" terminant par "dup.txt"
    let index = tmp.child(".xdg/data/nrip/index.json");
    let idx_str = std::fs::read_to_string(index.path())?;
    let v: serde_json::Value = serde_json::from_str(&idx_str)?;
    let entries = v
        .get("items")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();

    let mut trashed = vec![];
    for e in entries {
        if let Some(p) = e.get("trashed_path").and_then(|s| s.as_str()) {
            if p.ends_with("dup.txt") {
                trashed.push(p.to_string());
            }
        }
    }
    assert!(
        trashed.len() >= 2,
        "on attend >=2 entrées dup.txt, got {:?}",
        trashed
    );
    let mut unique = trashed.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(
        unique.len(),
        trashed.len(),
        "les cibles doivent être uniques"
    );

    Ok(())
}

#[test]
#[serial]
fn bury_creates_graveyard_dir_if_missing() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());

    let file = tmp.child("x.txt");
    file.write_str("x")?;

    // Le dossier n'existe pas encore
    tmp.child(".xdg/data/nrip/graveyard")
        .assert(predicate::path::missing());

    Command::cargo_bin("nrip")?
        .arg(file.path())
        .assert()
        .success();

    // Il doit être créé par nrip
    tmp.child(".xdg/data/nrip/graveyard")
        .assert(predicate::path::exists());
    Ok(())
}
