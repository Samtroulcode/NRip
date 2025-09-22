use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use serial_test::serial;
use std::process::Command;

mod util;

#[cfg(unix)]
#[test]
#[serial]
fn bury_and_resurrect_symlink() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    let target_file = tmp.child("target.txt");
    target_file.write_str("target content")?;
    
    let symlink = tmp.child("link.txt");
    
    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    // Créer un lien symbolique
    std::os::unix::fs::symlink(target_file.path(), symlink.path())?;
    
    symlink.assert(predicate::path::exists());

    // Enterrer le lien symbolique
    Command::cargo_bin("nrip")?
        .arg(symlink.path())
        .assert()
        .success();

    symlink.assert(predicate::path::missing());

    // Ressusciter le lien
    Command::cargo_bin("nrip")?
        .args(["-r", "link.txt", "-y"])
        .assert()
        .success();

    // Le lien devrait être restauré et pointer vers le bon endroit
    symlink.assert(predicate::path::exists());
    
    tmp.close()?;
    Ok(())
}

#[test]
#[serial]
fn bury_files_with_special_characters_in_names() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    
    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    // Noms avec caractères spéciaux
    let special_names = vec![
        "file with spaces.txt",
        "file-with-dashes.txt", 
        "file_with_underscores.txt",
        "file.with.dots.txt",
        "file(with)parens.txt",
        "file[with]brackets.txt",
        "file{with}braces.txt",
    ];

    for name in &special_names {
        let file = tmp.child(name);
        file.write_str("content")?;
        
        // Enterrer
        Command::cargo_bin("nrip")?
            .arg(file.path())
            .assert()
            .success();
            
        file.assert(predicate::path::missing());
        
        // Ressusciter
        Command::cargo_bin("nrip")?
            .args(["-r", name, "-y"])
            .assert()
            .success();
            
        file.assert(predicate::path::exists());
    }

    tmp.close()?;
    Ok(())
}

#[test]
#[serial]
fn bury_files_with_unicode_names() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    
    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    let unicode_names = vec![
        "fichier_français.txt",
        "файл.txt", // cyrillique
        "文件.txt", // chinois
        "ファイル.txt", // japonais
        "🦀_emoji.txt", // emoji
    ];

    for name in &unicode_names {
        let file = tmp.child(name);
        file.write_str("unicode content")?;
        
        Command::cargo_bin("nrip")?
            .arg(file.path())
            .assert()
            .success();
            
        file.assert(predicate::path::missing());
        
        // Ressusciter par nom partiel simple (sécurisé pour unicode)
        let safe_prefix = name.chars().take(3).collect::<String>();
        if !safe_prefix.is_empty() {
            Command::cargo_bin("nrip")?
                .args(["-r", &safe_prefix, "-y"])
                .assert()
                .success();
        }
    }

    tmp.close()?;
    Ok(())
}

#[test]
#[serial]
fn bury_very_long_filename() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    
    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    // Nom très long (mais pas trop pour éviter les limites du système de fichiers)
    let long_name = "a".repeat(100) + ".txt";
    let file = tmp.child(&long_name);
    file.write_str("content")?;
    
    Command::cargo_bin("nrip")?
        .arg(file.path())
        .assert()
        .success();
        
    file.assert(predicate::path::missing());
    
    // Ressusciter par match partiel
    Command::cargo_bin("nrip")?
        .args(["-r", &"a".repeat(10), "-y"])
        .assert()
        .success();

    tmp.close()?;
    Ok(())
}

#[test]
#[serial]
fn bury_hidden_files() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    let hidden_file = tmp.child(".hidden");
    let hidden_dir = tmp.child(".hidden_dir");
    
    hidden_file.write_str("hidden content")?;
    hidden_dir.create_dir_all()?;
    
    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    // Enterrer fichier caché
    Command::cargo_bin("nrip")?
        .arg(hidden_file.path())
        .assert()
        .success();
        
    hidden_file.assert(predicate::path::missing());
    
    // Enterrer dossier caché
    Command::cargo_bin("nrip")?
        .arg(hidden_dir.path())
        .assert()
        .success();
        
    hidden_dir.assert(predicate::path::missing());
    
    // Ressusciter
    Command::cargo_bin("nrip")?
        .args(["-r", ".hidden", "-y"])
        .assert()
        .success();
        
    Command::cargo_bin("nrip")?
        .args(["-r", ".hidden_dir", "-y"])
        .assert()
        .success();

    tmp.close()?;
    Ok(())
}

#[test]
#[serial]
fn bury_empty_directory() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    let empty_dir = tmp.child("empty");
    empty_dir.create_dir_all()?;
    
    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    Command::cargo_bin("nrip")?
        .arg(empty_dir.path())
        .assert()
        .success();
        
    empty_dir.assert(predicate::path::missing());
    
    // Ressusciter
    Command::cargo_bin("nrip")?
        .args(["-r", "empty", "-y"])
        .assert()
        .success();
        
    empty_dir.assert(predicate::path::exists());

    tmp.close()?;
    Ok(())
}