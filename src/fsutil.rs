use anyhow::{Context, Result};
use std::path::Path;

pub fn find_kslim_root() -> Result<camino::Utf8PathBuf> {
    let cwd = std::env::current_dir().context("current directory")?;
    let mut path =
        camino::Utf8PathBuf::from_path_buf(cwd).map_err(|_| anyhow::anyhow!("non-utf8 path"))?;

    loop {
        let candidate = path.join("kslim.toml");
        if candidate.exists() {
            return Ok(path);
        }
        if !path.pop() {
            break;
        }
    }
    anyhow::bail!("not inside a kslim project (no kslim.toml found)")
}

pub fn ensure_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

#[allow(dead_code)]
pub fn remove_dir_contents(dir: &Path) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            std::fs::remove_dir_all(&path)?;
        } else {
            std::fs::remove_file(&path)?;
        }
    }
    Ok(())
}
