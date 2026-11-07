//! `eselect world` — regen world from pacman explicit set.
//!
//! Moves `--regen-world-from-explicit` logic out of emerge into the
//! utility tool (as planned in the ToDo).

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use colored::Colorize;

use crate::paths::WORLD;
use crate::util::fs::atomic_write;
use crate::util::root::{confirm, require_root};

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

/// Add pacman explicit packages (`pacman -Qeq`) that are missing from world.
pub fn cmd_regen(pretend: bool, ask: bool) -> Result<()> {
    if !pretend {
        require_root("eselect world regen")?;
    }

    let explicit = pacman_explicit()?;
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

fn pacman_explicit() -> Result<Vec<String>> {
    let out = Command::new("pacman")
        .args(["-Qeq"])
        .output()
        .context("run pacman -Qeq")?;
    if !out.status.success() {
        bail!("pacman -Qeq failed");
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
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
