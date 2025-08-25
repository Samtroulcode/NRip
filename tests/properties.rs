use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use proptest::prelude::*;
use serial_test::serial;
use std::process::Command;

proptest! {
  #![proptest_config(ProptestConfig {
      cases: 64,
      // (facultatif) désactiver la persistence des échecs
      failure_persistence: Some(Box::new(proptest::test_runner::FileFailurePersistence::Off)),
      .. ProptestConfig::default()
  })]
  #[test]
  #[serial]
  fn bury_then_resurrect_restores_original_path(file_name in "[a-zA-Z0-9._ -]{1,40}") {
    let tmp = assert_fs::TempDir::new().unwrap();
    let src = tmp.child(&file_name);
    src.write_str("data").unwrap();

    // Isoler l'env
    unsafe {
      std::env::set_var("HOME", tmp.path());
      std::env::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
      std::env::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());
    }

    // bury
    Command::cargo_bin("nrip").unwrap()
        .arg(src.path())
        .assert()
        .success();

    // resurrect NON-INTERACTIF : on passe la cible + -y pour éviter le prompt
    Command::cargo_bin("nrip").unwrap()
        .args(["-r", file_name.as_str(), "-y"])
        .assert()
        .success();

    // Vérifie que le fichier est restauré
    src.assert(predicates::path::exists());
  }
}
