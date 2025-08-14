use std::fs::{self, File};
use std::io::Write;
use safe_backup::Context;

fn mk_temp_ctx(prefix: &str) -> Context {
    let mut dir = std::env::temp_dir();
    let uniq = format!("{}_{}", prefix, std::process::id());
    dir.push(uniq);
    fs::create_dir_all(&dir).unwrap();
    Context::new(&dir).unwrap()
}

#[test]
fn backup_valid_creates_bak_and_logs() {
    let ctx = mk_temp_ctx("sbk_backup_pure");
    let src_path = ctx.data_dir.join("sample.txt");
    let mut f = File::create(&src_path).unwrap();
    writeln!(f, "Hello world").unwrap();

    let bak = ctx.backup_file("sample.txt").unwrap();
    assert!(bak.exists(), "backup should exist");
    let log_s = fs::read_to_string(&ctx.log_path).unwrap();
    assert!(log_s.contains("backup:"), "log should contain backup entry");
    assert!(bak.starts_with(&ctx.backups_dir));
}

#[test]
fn restore_overwrites_target() {
    let ctx = mk_temp_ctx("sbk_restore_pure");

    let orig = ctx.data_dir.join("data.txt");
    let mut f = File::create(&orig).unwrap();
    writeln!(f, "ORIG").unwrap();
    let bak = ctx.backups_dir.join("data.txt.bak");
    let mut fb = File::create(&bak).unwrap();
    writeln!(fb, "FROM-BAK").unwrap();

    ctx.restore_file("data.txt").unwrap();
    let s = fs::read_to_string(orig).unwrap();
    assert!(s.contains("FROM-BAK"));
}

#[test]
fn malicious_input_rejected() {
    let ctx = mk_temp_ctx("sbk_mal_pure");

    let err = ctx.backup_file("../etc/passwd").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);

    let err2 = ctx.delete_file("bad/name").unwrap_err();
    assert_eq!(err2.kind(), std::io::ErrorKind::InvalidInput);
}
