use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use serial_test::serial;
use std::process::Command;
use which::which;
mod util;

#[test]
#[serial]
fn list_works_without_fzf() {
    let tmp = assert_fs::TempDir::new().unwrap();
    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    Command::cargo_bin("nrip")
        .unwrap()
        .arg("--list")
        .assert()
        .success();
}

#[test]
#[serial]
fn list_uses_fzf_when_available_via_resurrect_interactive() {
    // Ce test ne force pas fzf pour 'list' (qui n'en a pas besoin),
    // mais vérifie que si fzf est présent, on peut entrer en mode interactif de resurrect.
    if which("fzf").is_err() {
        eprintln!("skip: fzf not found");
        return;
    }
    let tmp = assert_fs::TempDir::new().unwrap();
    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    // enterre un fichier
    let f = tmp.child("a.txt");
    f.write_str("x").unwrap();
    Command::cargo_bin("nrip")
        .unwrap()
        .arg(f.path())
        .assert()
        .success();

    // lance resurrect SANS cible pour déclencher fzf ; comme on n’interagit pas,
    // on s'attend à un statut "succès" avec "Aborted." (fzf sort 130/1 → mappé en Ok(vec![]))
    Command::cargo_bin("nrip")
        .unwrap()
        .arg("--resurrect")
        .assert()
        .success(); // pas d’échec du binaire
}
