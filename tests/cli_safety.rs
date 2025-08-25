use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[cfg(unix)]
#[test]
fn refuses_root_without_force() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("nrip")?
        .arg("/")
        .assert()
        .failure()
        .stderr(predicate::str::contains("refusé").or(predicate::str::contains("denied")));
    Ok(())
}
