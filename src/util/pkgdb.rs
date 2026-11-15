//! Direct reader for alpm databases (no pacman / emerge in between).
//!
//! * local db: `/var/lib/pacman/local/<name>-<ver>/desc` (plain files)
//! * sync dbs: `/var/lib/pacman/sync/<repo>.db` — a tar archive compressed
//!   with zstd or gzip; every package is `<name>-<ver>/desc`.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

pub const SYNC_DIR: &str = "/var/lib/pacman/sync";
pub const LOCAL_DIR: &str = "/var/lib/pacman/local";

#[derive(Debug, Clone, Default)]
pub struct Pkg {
    pub name: String,
    pub version: String,
    pub desc: String,
    pub base: String,
}

/// Parse a `desc` file into `%FIELD%` -> lines.
pub fn parse_desc(text: &str) -> BTreeMap<String, Vec<String>> {
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut field: Option<String> = None;
    for line in text.lines() {
        let t = line.trim();
        if let Some(key) = t.strip_prefix('%').and_then(|r| r.strip_suffix('%')) {
            field = Some(key.to_string());
            map.entry(key.to_string()).or_default();
            continue;
        }
        if t.is_empty() {
            field = None;
            continue;
        }
        if let Some(f) = &field {
            map.entry(f.clone()).or_default().push(t.to_string());
        }
    }
    map
}

fn first(m: &BTreeMap<String, Vec<String>>, k: &str) -> String {
    m.get(k)
        .and_then(|v| v.first())
        .cloned()
        .unwrap_or_default()
}

fn pkg_from_desc(text: &str) -> Option<Pkg> {
    let m = parse_desc(text);
    let name = first(&m, "NAME");
    if name.is_empty() {
        return None;
    }
    Some(Pkg {
        version: first(&m, "VERSION"),
        desc: first(&m, "DESC"),
        base: first(&m, "BASE"),
        name,
    })
}

/// Names of repositories that have a `<repo>.db` in the sync dir.
pub fn sync_repos() -> Vec<String> {
    let mut v = Vec::new();
    if let Ok(rd) = fs::read_dir(SYNC_DIR) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if let Some(r) = n.strip_suffix(".db") {
                v.push(r.to_string());
            }
        }
    }
    v.sort();
    v
}

/// Open a decompressing reader by magic bytes (zstd / gzip / plain tar).
fn open_db(path: &Path) -> Result<Box<dyn Read>> {
    let mut f = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut magic = [0u8; 4];
    let n = f.read(&mut magic)?;
    f.seek(SeekFrom::Start(0))?;
    let m = &magic[..n];
    if m.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
        Ok(Box::new(zstd::stream::read::Decoder::new(f)?))
    } else if m.starts_with(&[0x1F, 0x8B]) {
        Ok(Box::new(flate2::read::GzDecoder::new(f)))
    } else if m.starts_with(&[0xFD, b'7', b'z', b'X']) || m.starts_with(b"BZh") {
        bail!(
            "{}: xz/bzip2 databases are not supported (zstd, gzip and plain tar are)",
            path.display()
        )
    } else {
        Ok(Box::new(f))
    }
}

/// All packages of one sync repository, sorted by name.
pub fn read_sync(repo: &str) -> Result<Vec<Pkg>> {
    if repo.is_empty() || repo.contains('/') || repo.starts_with('.') {
        bail!("invalid repository name '{}'", repo);
    }
    let path = PathBuf::from(SYNC_DIR).join(format!("{}.db", repo));
    if !path.is_file() {
        bail!(
            "no sync database for '{}' ({} missing) — synchronise first (emerge --sync)",
            repo,
            path.display()
        );
    }
    let mut ar = tar::Archive::new(open_db(&path)?);
    let mut out = Vec::new();
    for entry in ar
        .entries()
        .with_context(|| format!("read {}", path.display()))?
    {
        let mut entry = entry?;
        let is_desc = entry
            .path()?
            .file_name()
            .map(|f| f == "desc")
            .unwrap_or(false);
        if !is_desc {
            continue;
        }
        let mut text = String::new();
        if entry.read_to_string(&mut text).is_err() {
            continue;
        }
        if let Some(p) = pkg_from_desc(&text) {
            out.push(p);
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Installed packages from the local db.
#[allow(dead_code)]
pub fn read_local() -> Result<Vec<Pkg>> {
    let root = Path::new(LOCAL_DIR);
    if !root.is_dir() {
        bail!(
            "{} missing — is the alpm local database present?",
            LOCAL_DIR
        );
    }
    let mut out = Vec::new();
    for e in fs::read_dir(root)?.flatten() {
        if let Ok(t) = fs::read_to_string(e.path().join("desc")) {
            if let Some(p) = pkg_from_desc(&t) {
                out.push(p);
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desc_fields() {
        let t = "%NAME%\nfoo\n\n%VERSION%\n1.0-1\n\n%DESC%\nA tool\n\n%BASE%\nfoo\n";
        let p = pkg_from_desc(t).unwrap();
        assert_eq!(p.name, "foo");
        assert_eq!(p.version, "1.0-1");
        assert_eq!(p.desc, "A tool");
    }

    #[test]
    fn rejects_bad_repo_name() {
        assert!(read_sync("../etc").is_err());
        assert!(read_sync("").is_err());
    }
}
