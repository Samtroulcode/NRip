use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use serial_test::serial;
use std::process::Command;

mod util;

/// bury(pop o) puis bury(test) ; restore(pop o) doit aussi restaurer test (parent),
/// et un second restore("test") ne doit rien faire (index déjà nettoyé).
#[test]
#[serial]
fn restore_child_auto_includes_buried_parent_and_is_idempotent()
-> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;

    // Isolement XDG/HOME pour le cimetière/idx
    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    // Arbo initiale: test/popo
    let d = tmp.child("test");
    d.create_dir_all()?;
    let f = d.child("popo");
    f.write_str("content")?;

    // 1) bury enfant
    Command::cargo_bin("nrip")?.arg(f.path()).assert().success();

    f.assert(predicate::path::missing());
    d.assert(predicate::path::exists()); // parent encore présent

    // 2) bury parent
    Command::cargo_bin("nrip")?.arg(d.path()).assert().success();

    d.assert(predicate::path::missing());

    // 3) restore enfant SEUL → doit inclure le parent enterré automatiquement
    Command::cargo_bin("nrip")?
        .args(["-r", "popo", "-y"])
        .assert()
        .success();

    // Attendus: parent recréé + enfant restauré
    d.assert(predicate::path::exists());
    f.assert(predicate::path::exists());

    // 4) restore parent après coup → l'entrée doit avoir disparu (no-op), pas d'erreur
    Command::cargo_bin("nrip")?
        .args(["-r", "test", "-y"])
        .assert()
        .success();

    // Toujours là, pas de duplication ni de déplacement
    d.assert(predicate::path::exists());
    f.assert(predicate::path::exists());

    // Index doit être vide (plus d'items) — vérif souple: index.json contient "items": []
    let index = tmp.child(".xdg/data/nrip/index.json");
    let idx_str = std::fs::read_to_string(index.path())?;
    assert!(
        idx_str.contains(r#""items":[]"#) || idx_str.contains(r#""items": []"#),
        "index should be empty, got: {idx_str}"
    );

    tmp.close()?;
    Ok(())
}

/// Même scénario, mais on restaure d’abord le parent puis l’enfant.
/// Le parent revient sans l’enfant (normal), puis l’enfant revient dedans.
#[test]
#[serial]
fn restore_parent_then_child_also_works() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = assert_fs::TempDir::new()?;

    util::set_var("HOME", tmp.path());
    util::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    util::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    let d = tmp.child("test");
    d.create_dir_all()?;
    let f = d.child("popo");
    f.write_str("content")?;

    // bury enfant puis parent
    Command::cargo_bin("nrip")?.arg(f.path()).assert().success();
    Command::cargo_bin("nrip")?.arg(d.path()).assert().success();

    d.assert(predicate::path::missing());

    // restore parent d'abord
    Command::cargo_bin("nrip")?
        .args(["-r", "test", "-y"])
        .assert()
        .success();

    d.assert(predicate::path::exists());
    // l'enfant n'a pas été restauré automatiquement (sélection explicite = parent seul)
    f.assert(predicate::path::missing());

    // restore enfant ensuite
    Command::cargo_bin("nrip")?
        .args(["-r", "popo", "-y"])
        .assert()
        .success();

    // Attendus
    d.assert(predicate::path::exists());
    f.assert(predicate::path::exists());

    Ok(())
}
