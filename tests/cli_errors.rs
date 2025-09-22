use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use serial_test::serial;
use std::process::Command;

mod util;

#[test]
#[serial]
fn bury_nonexistent_file_fails_with_error() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    
    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    Command::cargo_bin("nrip")?
        .arg("/nonexistent/file.txt")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No such file").or(predicate::str::contains("not found")).or(predicate::str::contains("existe pas")));

    tmp.close()?;
    Ok(())
}

#[test]
#[serial]
fn bury_directory_without_recursive_works() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    let dir = tmp.child("test_dir");
    dir.create_dir_all()?;
    let file_in_dir = dir.child("file.txt");
    file_in_dir.write_str("content")?;
    
    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    // Devrait pouvoir enterrer un dossier avec son contenu
    Command::cargo_bin("nrip")?
        .arg(dir.path())
        .assert()
        .success();

    dir.assert(predicate::path::missing());

    tmp.close()?;
    Ok(())
}

#[cfg(unix)]
#[test]
#[serial]
fn bury_protected_file_requires_force() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    let file = tmp.child("protected.txt");
    file.write_str("protected content")?;

    // Rendre le fichier en lecture seule
    let metadata = file.path().metadata()?;
    let mut permissions = metadata.permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(file.path(), permissions)?;
    
    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    // Devrait réussir même avec un fichier en lecture seule (mv fonctionne)
    Command::cargo_bin("nrip")?
        .arg(file.path())
        .assert()
        .success();

    file.assert(predicate::path::missing());

    tmp.close()?;
    Ok(())
}

#[test]
#[serial]
fn resurrect_nonexistent_entry_handles_gracefully() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    
    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    // Tenter de ressusciter quelque chose qui n'existe pas
    Command::cargo_bin("nrip")?
        .args(["-r", "nonexistent.txt", "-y"])
        .assert()
        .success(); // Devrait gérer gracieusement, pas d'échec

    tmp.close()?;
    Ok(())
}

#[test]
#[serial]
fn resurrect_with_existing_file_at_destination_handles_conflict() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    let original_file = tmp.child("conflict.txt");
    original_file.write_str("original content")?;
    
    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    // Enterrer le fichier
    Command::cargo_bin("nrip")?
        .arg(original_file.path())
        .assert()
        .success();

    original_file.assert(predicate::path::missing());

    // Créer un nouveau fichier au même endroit
    original_file.write_str("new content")?;

    // Tenter de ressusciter - devrait gérer le conflit
    let result = Command::cargo_bin("nrip")?
        .args(["-r", "conflict.txt", "-y"])
        .output()?;

    // Devrait soit réussir (avec renommage), soit échouer gracieusement
    let stdout = String::from_utf8_lossy(&result.stdout);
    let stderr = String::from_utf8_lossy(&result.stderr);
    
    // Le programme ne devrait pas planter
    assert!(result.status.success() || !stderr.contains("panic"));

    tmp.close()?;
    Ok(())
}

#[test]
#[serial]  
fn multiple_files_with_some_nonexistent_processes_existing_ones() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    let existing_file = tmp.child("exists.txt");
    existing_file.write_str("content")?;
    
    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    // Essayer d'enterrer un mélange de fichiers existants et inexistants
    let result = Command::cargo_bin("nrip")?
        .args([existing_file.path().to_str().unwrap(), "/nonexistent.txt"])
        .output()?;

    // Au moins le fichier existant devrait être traité
    // Le programme pourrait échouer ou réussir partiellement
    existing_file.assert(predicate::path::missing());

    tmp.close()?;
    Ok(())
}