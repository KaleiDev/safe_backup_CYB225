use anyhow::{Context, Result};
use std::path::Path;
use fs_err as fs;
use tempfile::NamedTempFile;
use path_absolutize::Absolutize;

/// Copy src -> dest using a temporary file + atomic rename
pub fn atomic_copy(src: &Path, dest: &Path) -> Result<()> {
    let parent = dest.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let mut tmp = NamedTempFile::new_in(parent)?;
    {
        let mut src_f = fs::File::open(src)
            .with_context(|| format!("opening {}", src.display()))?;
        std::io::copy(&mut src_f, &mut tmp)
            .with_context(|| format!("copying {} -> tmp", src.display()))?;
    }
    tmp.persist(dest).with_context(|| format!("persisting tmp to {}", dest.display()))?;
    Ok(())
}

/// Atomically overwrite `target` with the contents of `src`
pub fn atomic_overwrite(src: &Path, target: &Path) -> Result<()> {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let mut tmp = NamedTempFile::new_in(parent)?;
    {
        let mut src_f = fs::File::open(src)?;
        std::io::copy(&mut src_f, &mut tmp)?;
    }
    // Rename over target
    tmp.persist(target)?;
    Ok(())
}

/// Ensure `child` is within `parent_dir` (prevents path traversal tricks)
pub fn ensure_within(child: &Path, parent_dir: &Path) -> Result<()> {
    let child_abs = child.absolutize()?;
    let parent_abs = parent_dir.absolutize()?;
    anyhow::ensure!(
        child_abs.starts_with(&*parent_abs),
        "Path {} escapes parent {}",
        child_abs.display(),
        parent_abs.display()
    );
    Ok(())
}

pub fn read_to_string_lossy(path: &Path) -> Result<String> {
    let data = fs::read(path)?;
    Ok(String::from_utf8_lossy(&data).to_string())
}
