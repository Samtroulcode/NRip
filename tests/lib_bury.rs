use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use serial_test::serial;
use std::process::Command;
mod util;

#[test]
#[serial]
fn bury_twice_same_name_produces_distinct_targets() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;
    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    let a = tmp.child("dup.txt");
    a.write_str("A")?;
    let b = tmp.child("dup.txt"); // même nom après que A sera déplacé
    b.write_str("B")?;

    // enterre A
    Command::cargo_bin("nrip")?.arg(a.path()).assert().success();
    // enterre B
    Command::cargo_bin("nrip")?.arg(b.path()).assert().success();

    // lis le graveyard et vérifie qu'il y a au moins deux entrées distinctes pour dup.txt
    let gy = tmp.child(".xdg/data/nrip/graveyard");
    let mut hits = vec![];
    for entry in std::fs::read_dir(gy.path())? {
        let e = entry?;
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with("dup") {
            hits.push(name);
        }
    }
    assert!(
        hits.len() >= 2,
        "on attend au moins 2 cibles distinctes dans le graveyard"
    );
    assert!(
        hits.windows(2).all(|w| w[0] != w[1]),
        "les noms doivent être uniques"
    );

    Ok(())
}
