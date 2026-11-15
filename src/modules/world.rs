//! `eselect world` — show / regen world from the local alpm explicit set.
//!
//! Reads `/var/lib/pacman/local/*/desc` directly (no `pacman` CLI).
//! Explicit packages are those without `%REASON%` or with reason `0`.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use colored::Colorize;

use crate::paths::WORLD;
use crate::util::fs::atomic_write;
use crate::util::root::{confirm, require_root};

const LOCAL_DB: &str = "/var/lib/pacman/local";

pub fn cmd_show() -> Result<()> {
    let path = Path::new(WORLD);
    if !path.is_file() {
        println!("{} does not exist", WORLD);
        return Ok(());
    }
    let text = fs::read_to_string(path)?;
    let mut n = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        println!("{}", t);
        n += 1;
    }
    println!("# {} entries", n);
    Ok(())
}

/// Add installed explicit packages (local alpm DB) that are missing from world.
pub fn cmd_regen(pretend: bool, ask: bool) -> Result<()> {
    if !pretend {
        require_root("eselect world regen")?;
    }

    let explicit = local_explicit()?;
    let current = load_world()?;
    let mut missing: Vec<String> = explicit
        .into_iter()
        .filter(|p| !current.contains(p))
        .collect();
    missing.sort();

    if missing.is_empty() {
        println!(
            "{} world already covers all explicit packages",
            ">>>".green().bold()
        );
        return Ok(());
    }

    println!(
        "{} {} package(s) to add to world:",
        ">>>".yellow().bold(),
        missing.len()
    );
    for p in &missing {
        println!("  {}", p);
    }

    if pretend {
        println!("(pretend — no changes)");
        return Ok(());
    }
    if ask && !confirm("Append these to world?") {
        println!("aborted");
        return Ok(());
    }

    // Append safely: keep a missing trailing newline from gluing two names.
    let mut text = fs::read_to_string(WORLD).unwrap_or_default();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    for p in &missing {
        text.push_str(p);
        text.push('\n');
    }
    atomic_write(Path::new(WORLD), &text).with_context(|| format!("write {}", WORLD))?;
    println!(
        "{} appended {} entries to {}",
        ">>>".green().bold(),
        missing.len(),
        WORLD
    );
    Ok(())
}

/// Explicit packages from the local alpm database (no package-manager CLI).
///
/// `%REASON%` absent or `0` → explicit; `1` → installed as a dependency.
fn local_explicit() -> Result<Vec<String>> {
    let root = Path::new(LOCAL_DB);
    if !root.is_dir() {
        bail!("{} missing — is the alpm local database present?", LOCAL_DB);
    }
    let mut out = Vec::new();
    for e in fs::read_dir(root).with_context(|| LOCAL_DB.to_string())? {
        let e = e?;
        if !e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let desc = e.path().join("desc");
        if !desc.is_file() {
            continue;
        }
        let text = fs::read_to_string(&desc).unwrap_or_default();
        let (name, reason) = parse_desc_name_reason(&text);
        let Some(name) = name else { continue };
        // Explicit: no REASON field, or REASON == 0.
        let is_explicit = match reason.as_deref() {
            None => true,
            Some(r) => r.trim() == "0",
        };
        if is_explicit {
            out.push(name);
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn parse_desc_name_reason(text: &str) -> (Option<String>, Option<String>) {
    let mut name: Option<String> = None;
    let mut reason: Option<String> = None;
    let mut field: Option<&str> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix('%') {
            if let Some(key) = rest.strip_suffix('%') {
                field = Some(key);
                continue;
            }
        }
        let t = line.trim();
        if t.is_empty() {
            field = None;
            continue;
        }
        match field {
            Some("NAME") if name.is_none() => name = Some(t.to_string()),
            Some("REASON") if reason.is_none() => reason = Some(t.to_string()),
            _ => {}
        }
    }
    (name, reason)
}

fn load_world() -> Result<HashSet<String>> {
    let path = Path::new(WORLD);
    if !path.is_file() {
        return Ok(HashSet::new());
    }
    let text = fs::read_to_string(path)?;
    let mut set = HashSet::new();
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        // strip repo prefix aur/ abs/ repo/
        let bare = t.split('/').next_back().unwrap_or(t);
        set.insert(bare.to_string());
        set.insert(t.to_string());
    }
    Ok(set)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_explicit_without_reason() {
        let t = "%NAME%\nfoo\n%VERSION%\n1.0-1\n";
        let (n, r) = parse_desc_name_reason(t);
        assert_eq!(n.as_deref(), Some("foo"));
        assert!(r.is_none());
    }

    #[test]
    fn parses_dep_reason() {
        let t = "%NAME%\nbar\n%REASON%\n1\n";
        let (n, r) = parse_desc_name_reason(t);
        assert_eq!(n.as_deref(), Some("bar"));
        assert_eq!(r.as_deref(), Some("1"));
    }
}
