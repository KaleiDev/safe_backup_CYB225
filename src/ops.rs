use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;
use std::path::{Path, PathBuf};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use fs_err as fs;
use path_absolutize::Absolutize;
use crate::fsx::{atomic_copy, atomic_overwrite, ensure_within, read_to_string_lossy};



/// A backup entry filename format:
/// <hex_sha256_of_original_abs_path>__<timestamp_rfc3339>__<basename>
/// Example:
///   9f...c3__2025-08-14T10:22:11Z__data.txt
fn make_backup_id(original: &Path) -> Result<String> {
    let abs = original
        .absolutize()
        .context("failed to absolutize original path")?;
    let mut hasher = Sha256::new();
    hasher.update(abs.as_os_str().to_string_lossy().as_bytes());
    let hash = hex::encode(hasher.finalize());
    let ts = OffsetDateTime::now_utc().format(&Rfc3339).unwrap_or_else(|_| "now".into());
    let base = original.file_name().and_then(|s| s.to_str()).unwrap_or("file");
    Ok(format!("{hash}__{ts}__{base}"))
}

/// Returns Ok(true) if the file exists and is a normal file
fn is_file(p: &Path) -> bool {
    fs::metadata(p).map(|m| m.is_file()).unwrap_or(false)
}

pub fn backup(original: &Path, backup_dir: &Path) -> Result<()> {
    anyhow::ensure!(is_file(original), "Original file does not exist or is not a regular file: {}", original.display());
    fs::create_dir_all(backup_dir)?;

    let id = make_backup_id(original)?;
    let dest = backup_dir.join(&id);

    atomic_copy(original, &dest).with_context(|| format!("Backing up {} to {}", original.display(), dest.display()))?;

    let checksum = file_sha256(&dest)?;
    println!(
        "BACKED UP: id={id} path={} size={}B sha256={}",
        dest.display(),
        fs::metadata(&dest)?.len(),
        checksum
    );
    Ok(())
}

pub fn list(original: &Path, backup_dir: &Path) -> Result<()> {
    let abs_hash_prefix = {
        let abs = original.absolutize().context("failed to absolutize path")?;
        let mut hasher = Sha256::new();
        hasher.update(abs.as_os_str().to_string_lossy().as_bytes());
        hex::encode(hasher.finalize())
    };

    let mut entries: Vec<PathBuf> = WalkDir::new(backup_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| p.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.starts_with(&abs_hash_prefix))
            .unwrap_or(false)
        )
        .collect();

    entries.sort();

    if entries.is_empty() {
        println!("No backups found for {}", original.display());
        return Ok(());
    }

    for p in entries {
        let id = p.file_name().unwrap().to_string_lossy().to_string();
        let size = fs::metadata(&p)?.len();
        let sha = file_sha256(&p)?;
        println!(
            "id={id} size={}B sha256={sha} backup={} original={}",
            size,
            p.display(),
            original.display()
        );
    }
    Ok(())
}

pub fn restore(original: &Path, id: Option<&str>, backup_dir: &Path) -> Result<()> {
    fs::create_dir_all(backup_dir)?;

    let candidate = match id {
        Some(id) => backup_dir.join(id),
        None => latest_backup_for(original, backup_dir)?
            .context("No backups found to restore")?,
    };

    anyhow::ensure!(is_file(&candidate), "Backup not found: {}", candidate.display());
    // Prevent restoring outside of the original's parent dir via traversal
    let _target_dir = original.parent().unwrap_or_else(|| Path::new("."));
    ensure_within(&candidate, backup_dir).context("Backup file must be within the backup directory")?;

    // Atomic write via temp file then rename
    atomic_overwrite(&candidate, original)?;

    println!("RESTORED: {} <- {}", original.display(), candidate.display());
    Ok(())
}

pub fn delete(id: &str, backup_dir: &Path) -> Result<()> {
    let target = backup_dir.join(id);
    anyhow::ensure!(is_file(&target), "Backup not found: {}", target.display());
    fs::remove_file(&target)?;
    println!("DELETED: {}", target.display());
    Ok(())
}

fn latest_backup_for(original: &Path, backup_dir: &Path) -> Result<Option<PathBuf>> {
    let mut matches: Vec<PathBuf> = vec![];
    let abs = original.absolutize().context("failed to absolutize path")?;
    let mut hasher = Sha256::new();
    hasher.update(abs.as_os_str().to_string_lossy().as_bytes());
    let prefix = hex::encode(hasher.finalize());

    for entry in fs::read_dir(backup_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() { continue; }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(&prefix) {
            matches.push(entry.path());
        }
    }
    matches.sort();
    Ok(matches.pop())
}

fn file_sha256(path: &Path) -> Result<String> {
    use std::io::Read;
    let mut hasher = Sha256::new();
    let mut f = fs::File::open(path)?;
    let mut buf = [0u8; 8192];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn view(original: &Path, id: Option<&str>, backup_dir: &Path) -> Result<()> {
    let target_path = match id {
        Some(id) => backup_dir.join(id),
        None => original.to_path_buf(),
    };

    anyhow::ensure!(fs::metadata(&target_path)?.is_file(), "File not found: {}", target_path.display());

    let contents = read_to_string_lossy(&target_path)?;
    println!("--- BEGIN CONTENTS ({}) ---", target_path.display());
    println!("{}", contents);
    println!("--- END CONTENTS ---");
    Ok(())
}
