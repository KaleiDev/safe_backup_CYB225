use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*;
use std::process::Command;
use std::fs;

#[test]
fn backup_and_restore_cycle() -> Result<(), Box<dyn std::error::Error>> {
    let proj_root = env!("CARGO_MANIFEST_DIR");
    let original = format!("{proj_root}/tests/fixtures/data_test/data.txt");

    // Ensure backups dir is clean
    let _ = fs::remove_dir_all(format!("{proj_root}/backups"));

    // backup
    Command::cargo_bin("safe_backup_rust")?
        .args(["backup", &original])
        .assert()
        .success();

    // list
    Command::cargo_bin("safe_backup_rust")?
        .args(["list", &original])
        .assert()
        .success()
        .stdout(predicate::str::contains("id="));

    Ok(())
}
