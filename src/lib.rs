//! SafeBackup core library (pure Rust, folder-sandboxed)
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Context {
    pub base_dir: PathBuf,
    pub data_dir: PathBuf,
    pub backups_dir: PathBuf,
    pub log_path: PathBuf,
}

impl Context {
    /// Initialize a context rooted at `base_dir` with subfolders:
    /// - data_test/ : primary files
    /// - backups/   : backup files (<name>.bak)
    /// - logs/      : logfile.txt
    /// You may override the data dir via SAFE_BACKUP_DATA_DIR env var.
    pub fn new(base_dir: impl AsRef<Path>) -> io::Result<Self> {
        let base = base_dir.as_ref().to_path_buf();
        let data_dir_name = std::env::var("SAFE_BACKUP_DATA_DIR").unwrap_or_else(|_| "data_test".to_string());
        let data_dir = base.join(data_dir_name);
        let backups_dir = base.join("backups");
        let logs_dir = base.join("logs");
        let log_path = logs_dir.join("logfile.txt");

        fs::create_dir_all(&data_dir)?;
        fs::create_dir_all(&backups_dir)?;
        fs::create_dir_all(&logs_dir)?;

        // touch log
        OpenOptions::new().create(true).append(true).open(&log_path)?;

        Ok(Self { base_dir: base, data_dir, backups_dir, log_path })
    }

    /// Append a timestamped entry to the log file.
    pub fn log(&self, action: &str) -> io::Result<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown-time".into());
        let mut f = OpenOptions::new().create(true).append(true).open(&self.log_path)?;
        writeln!(f, "[{now}] {action}")?;
        Ok(())
    }

    /// Allow only a safe subset of filename characters (no separators, traversal, or reserved names).
    pub fn validate_filename(&self, name: &str) -> io::Result<()> {
        if name.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty file name"));
        }
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_') {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "unsupported characters in file name"));
        }
        if name == "." || name == ".." || name.contains(std::path::MAIN_SEPARATOR) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid file name"));
        }
        Ok(())
    }

    fn data_path(&self, name: &str) -> PathBuf {
        self.data_dir.join(name)
    }

    fn backup_path(&self, name: &str) -> PathBuf {
        self.backups_dir.join(format!("{name}.bak"))
    }

    fn reject_symlink(&self, path: &Path) -> io::Result<()> {
        let meta = fs::symlink_metadata(path)?;
        if meta.file_type().is_symlink() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "symlinks are not allowed"));
        }
        Ok(())
    }

    fn ensure_within(&self, path: &Path, root: &Path) -> io::Result<()> {
        // Canonicalize both sides to avoid macOS /var -> /private/var mismatch (and similar cases).
        let real = fs::canonicalize(path)?;
        let root_real = fs::canonicalize(root)?;
        if !real.starts_with(&root_real) {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "path escapes sandbox"));
        }
        Ok(())
    }

    /// Backup: copy `data_test/<name>` -> `backups/<name>.bak`
    pub fn backup_file(&self, name: &str) -> io::Result<PathBuf> {
        self.validate_filename(name)?;
        let src = self.data_path(name);
        if !src.exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "source file does not exist"));
        }
        self.reject_symlink(&src)?;
        self.ensure_within(&src, &self.data_dir)?;

        let dst = self.backup_path(name);
        fs::copy(&src, &dst)?;
        self.log(&format!("backup: {} -> {}", src.display(), dst.display()))?;
        Ok(dst)
    }

    /// Restore: copy `backups/<name>.bak` -> `data_test/<name>` (overwrite allowed).
    pub fn restore_file(&self, name: &str) -> io::Result<PathBuf> {
        self.validate_filename(name)?;
        let src = self.backup_path(name);
        if !src.exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "backup file does not exist"));
        }
        self.reject_symlink(&src)?;
        self.ensure_within(&src, &self.backups_dir)?;

        let dst = self.data_path(name);
        fs::copy(&src, &dst)?;
        self.log(&format!("restore: {} -> {}", src.display(), dst.display()))?;
        Ok(dst)
    }

    /// Delete: remove `data_test/<name>`.
    pub fn delete_file(&self, name: &str) -> io::Result<()> {
        self.validate_filename(name)?;
        let target = self.data_path(name);
        if !target.exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "file not found"));
        }
        self.reject_symlink(&target)?;
        self.ensure_within(&target, &self.data_dir)?;

        fs::remove_file(&target)?;
        self.log(&format!("delete: {}", target.display()))?;
        Ok(())
    }
}
