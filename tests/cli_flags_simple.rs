use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use serial_test::serial;
use std::process::Command;

mod util;

#[test]
#[serial]
fn dry_run_flag_exists_and_works() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    let file = tmp.child("test.txt");
    file.write_str("content")?;

    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    // Test que --dry-run est accepté par le CLI
    Command::cargo_bin("nrip")?
        .args(["--dry-run", file.path().to_str().unwrap()])
        .assert()
        .success();

    tmp.close()?;
    Ok(())
}

#[test]
#[serial]
fn force_flag_exists_and_works() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    let file = tmp.child("test.txt");
    file.write_str("content")?;

    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    // Test que --force est accepté par le CLI
    Command::cargo_bin("nrip")?
        .args(["--force", file.path().to_str().unwrap()])
        .assert()
        .success();

    file.assert(predicate::path::missing());

    tmp.close()?;
    Ok(())
}

#[test]
#[serial]
fn yes_flag_works_with_cremate() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    let file = tmp.child("test.txt");
    file.write_str("content")?;

    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    // Enterrer d'abord
    Command::cargo_bin("nrip")?.arg(file.path()).assert().success();

    // Cremate avec --yes devrait procéder sans demander confirmation
    Command::cargo_bin("nrip")?
        .args(["-c", "test.txt", "--yes"])
        .assert()
        .success();

    tmp.close()?;
    Ok(())
}

#[test]
#[serial]
fn yes_flag_works_with_resurrect() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    let file = tmp.child("test.txt");
    file.write_str("content")?;

    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    // Enterrer d'abord
    Command::cargo_bin("nrip")?.arg(file.path()).assert().success();
    file.assert(predicate::path::missing());

    // Resurrect avec --yes
    Command::cargo_bin("nrip")?
        .args(["-r", "test.txt", "--yes"])
        .assert()
        .success();

    file.assert(predicate::path::exists());

    tmp.close()?;
    Ok(())
}