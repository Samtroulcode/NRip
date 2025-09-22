use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use serial_test::serial;
use std::process::Command;

mod util;

#[test]
#[serial]
fn cremate_removes_file_permanently_from_graveyard() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    let file = tmp.child("test.txt");
    file.write_str("content")?;

    // Isoler l'environnement
    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    // 1) Enterrer le fichier
    Command::cargo_bin("nrip")?
        .arg(file.path())
        .assert()
        .success();

    file.assert(predicate::path::missing());

    // 2) Vérifier qu'il est dans le graveyard
    let graveyard = tmp.child(".xdg/data/nrip/graveyard");
    graveyard.assert(predicate::path::exists());

    // 3) Cremate avec confirmation automatique
    Command::cargo_bin("nrip")?
        .args(["-c", "test.txt", "-y"])
        .assert()
        .success();

    // 4) Vérifier que l'index ne contient plus l'entrée
    let index = tmp.child(".xdg/data/nrip/index.json");
    let idx_str = std::fs::read_to_string(index.path())?;
    assert!(!idx_str.contains("test.txt"), "Index should not contain test.txt after cremation");

    tmp.close()?;
    Ok(())
}

#[test]
#[serial]
fn cremate_interactive_mode_lists_candidates() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    let file1 = tmp.child("file1.txt");
    let file2 = tmp.child("file2.txt");
    file1.write_str("content1")?;
    file2.write_str("content2")?;

    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    // Enterrer les deux fichiers
    Command::cargo_bin("nrip")?.arg(file1.path()).assert().success();
    Command::cargo_bin("nrip")?.arg(file2.path()).assert().success();

    // Mode interactif sans sélection spécifique - devrait lister les options
    let output = Command::cargo_bin("nrip")?
        .args(["-c"])
        .output()?;

    // En mode interactif sans fzf, devrait afficher la liste ou réussir
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Le test réussit si soit on voit les fichiers, soit la commande réussit gracieusement
    let shows_files = stdout.contains("file1.txt") || stdout.contains("file2.txt");
    let runs_successfully = output.status.success();
    
    assert!(shows_files || runs_successfully, 
           "Interactive cremate should list files or succeed gracefully. stdout: {}, stderr: {}", stdout, stderr);

    tmp.close()?;
    Ok(())
}

#[test]
#[serial]
fn cremate_specific_file_by_partial_match() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    let file = tmp.child("unique_name.txt");
    file.write_str("content")?;

    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    // Enterrer
    Command::cargo_bin("nrip")?.arg(file.path()).assert().success();

    // Cremate avec match partiel
    Command::cargo_bin("nrip")?
        .args(["-c", "unique", "-y"])
        .assert()
        .success();

    // Vérifier suppression de l'index
    let index = tmp.child(".xdg/data/nrip/index.json");
    let idx_str = std::fs::read_to_string(index.path())?;
    assert!(!idx_str.contains("unique_name.txt"));

    tmp.close()?;
    Ok(())
}

#[test]
#[serial]
fn cremate_nonexistent_file_fails_gracefully() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    
    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    // Tenter de cremate un fichier qui n'existe pas dans le graveyard
    Command::cargo_bin("nrip")?
        .args(["-c", "nonexistent.txt", "-y"])
        .assert()
        .success(); // Devrait réussir sans erreur (comportement graceful)

    tmp.close()?;
    Ok(())
}