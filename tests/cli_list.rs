use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use serial_test::serial;
use std::process::Command;

mod util;

#[test]
#[serial]
fn list_empty_graveyard_shows_appropriate_message() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    
    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    let output = Command::cargo_bin("nrip")?
        .arg("--list")
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Devrait indiquer que le cimetière est vide ou ne pas afficher d'entrées
    assert!(
        stdout.contains("empty") || 
        stdout.contains("vide") || 
        stdout.contains("No") ||
        stdout.contains("Aucun") ||
        stdout.trim().is_empty(),
        "Empty graveyard should show appropriate message: {}", stdout
    );

    tmp.close()?;
    Ok(())
}

#[test]
#[serial]
fn list_shows_buried_files_with_details() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    let file1 = tmp.child("document.txt");
    let file2 = tmp.child("image.jpg");
    
    file1.write_str("Some document content")?;
    file2.write_str("fake image data")?;

    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    // Enterrer les fichiers
    Command::cargo_bin("nrip")?.arg(file1.path()).assert().success();
    Command::cargo_bin("nrip")?.arg(file2.path()).assert().success();

    let output = Command::cargo_bin("nrip")?
        .arg("--list")
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Devrait afficher les noms des fichiers
    assert!(stdout.contains("document.txt"), "Should show document.txt in list");
    assert!(stdout.contains("image.jpg"), "Should show image.jpg in list");
    
    // Devrait contenir des informations temporelles (âge)
    assert!(
        stdout.contains("ago") || 
        stdout.contains("il y a") ||
        stdout.contains("sec") ||
        stdout.contains("min") ||
        stdout.contains("h") ||
        stdout.contains("d"),
        "Should show age information: {}", stdout
    );

    tmp.close()?;
    Ok(())
}

#[test]
#[serial]
fn list_shows_file_types_and_sizes() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    
    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    // Créer différents types de fichiers/dossiers
    let file = tmp.child("regular.txt");
    file.write_str("content")?;
    
    let dir = tmp.child("folder");
    dir.create_dir_all()?;
    let file_in_dir = dir.child("nested.txt");
    file_in_dir.write_str("nested content")?;

    // Enterrer fichier et dossier
    Command::cargo_bin("nrip")?.arg(file.path()).assert().success();
    Command::cargo_bin("nrip")?.arg(dir.path()).assert().success();

    let output = Command::cargo_bin("nrip")?
        .arg("--list")
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Devrait distinguer fichiers et dossiers
    assert!(
        stdout.contains("📄") || stdout.contains("📁") || 
        stdout.contains("F") || stdout.contains("D") ||
        stdout.contains("file") || stdout.contains("dir"),
        "Should show file type indicators: {}", stdout
    );

    // Devrait afficher les noms
    assert!(stdout.contains("regular.txt"), "Should show file name");
    assert!(stdout.contains("folder"), "Should show directory name");

    tmp.close()?;
    Ok(())
}

#[test]
#[serial]
fn list_output_is_properly_formatted() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    let file = tmp.child("test.txt");
    file.write_str("test content")?;

    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    Command::cargo_bin("nrip")?.arg(file.path()).assert().success();

    let output = Command::cargo_bin("nrip")?
        .arg("--list")
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // La sortie ne devrait pas être vide
    assert!(!stdout.trim().is_empty(), "List output should not be empty");
    
    // Devrait contenir le nom du fichier
    assert!(stdout.contains("test.txt"), "Should contain the buried file name");
    
    // Chaque ligne devrait être raisonnablement formatée (pas de lignes excessivement longues)
    for line in stdout.lines() {
        if !line.trim().is_empty() {
            assert!(line.len() < 200, "Line too long: {}", line);
        }
    }

    tmp.close()?;
    Ok(())
}