use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use proptest::prelude::*;
use serial_test::serial;

proptest! {
  #![proptest_config(ProptestConfig::with_cases(64))]
  #[test]
  #[serial]
  fn bury_then_resurrect_restores_original_path(file_name in "[a-zA-Z0-9._ -]{1,40}") {
    let tmp = assert_fs::TempDir::new().unwrap();
    let src = tmp.child(&file_name);
    src.write_str("data").unwrap();

    std::env::set_var("HOME", tmp.path());
    std::env::set_var("XDG_DATA_HOME", tmp.child(".xdg/data").path());
    std::env::set_var("XDG_CONFIG_HOME", tmp.child(".xdg/config").path());

    std::process::Command::cargo_bin("nrip").unwrap()
        .arg(src.path())
        .assert()
        .success();

    std::process::Command::cargo_bin("nrip").unwrap()
        .args(["--resurrect"])
        .assert()
        .success();

    src.assert(predicates::path::exists());
  }
}
