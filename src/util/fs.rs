//! Path / file helpers.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn ensure_dir(path: &Path) -> io::Result<()> {
    if path.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(path)
}

/// Write `data` to `path` via tmp file + rename (same dir).
pub fn atomic_write(path: &Path, data: &str) -> io::Result<()> {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    let tmp = path.with_file_name(name);
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

/// True only for a real directory (a symlink to a dir is NOT one).
pub fn is_real_dir(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.file_type().is_dir())
        .unwrap_or(false)
}

/// Relative path from `from` dir to `to` (for symlink values).
pub fn relative_from(from: &Path, to: &Path) -> Option<PathBuf> {
    let from = fs::canonicalize(from).ok()?;
    let to = fs::canonicalize(to).ok()?;
    pathdiff(&from, &to)
}

fn pathdiff(from: &Path, to: &Path) -> Option<PathBuf> {
    let from_c: Vec<_> = from.components().collect();
    let to_c: Vec<_> = to.components().collect();
    let mut i = 0;
    while i < from_c.len() && i < to_c.len() && from_c[i] == to_c[i] {
        i += 1;
    }
    let mut out = PathBuf::new();
    for _ in i..from_c.len() {
        out.push("..");
    }
    for c in &to_c[i..] {
        out.push(c.as_os_str());
    }
    if out.as_os_str().is_empty() {
        out.push(".");
    }
    Some(out)
}
