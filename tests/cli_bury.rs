use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use serial_test::serial;
use std::process::Command;
mod util;

#[test]
#[serial]
fn bury_moves_file_to_graveyard_and_updates_index() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    let file = tmp.child("hello.txt");
    file.write_str("hi")?;

    // On isole GRAVEYARD/INDEX via XDG_DATA_HOME
    let data = tmp.child(".local/share");
    util::set_var("XDG_DATA_HOME", data.path());

    Command::cargo_bin("nrip")? // auto-résolution du binaire de ce crate
        .arg(file.path())
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    // Le src a disparu
    file.assert(predicate::path::missing());

    // Le fichier est dans le graveyard
    let gy = data.child("nrip/graveyard");
    gy.assert(predicate::path::exists());

    // L’index contient une entrée qui pointe vers hello.txt
    let index = data.child("nrip/index.json");
    let idx_str = std::fs::read_to_string(index.path())?;
    assert!(idx_str.contains("hello.txt"));

    tmp.close()?;
    Ok(())
}
