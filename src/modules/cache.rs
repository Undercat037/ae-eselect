//! `eselect cache` — clean package cache and AE per-user caches.
//!
//! Package cache: `/var/cache/pacman/pkg` (filesystem only, no package-manager CLI).
//! AE layout (per user): ~/.cache/aura-emerge/{build,sources,packages}.
//! Running as root we must not follow symlinks planted by users, so every
//! path component below the home dir is checked to be a real directory.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use colored::Colorize;

use crate::paths::{AE_USER_CACHE, PACMAN_CACHE};
use crate::util::fs::is_real_dir;
use crate::util::root::{confirm, require_root};

pub struct Opts {
    pub pacman: bool,
    pub sources: bool,
    pub packages: bool,
    pub build: bool,
    pub all: bool,
    pub pretend: bool,
    pub yes: bool,
}

pub fn cmd_clean(o: &Opts) -> Result<()> {
    if !o.pretend {
        require_root("eselect cache clean")?;
    }
    let do_pacman = o.all || o.pacman;
    let do_sources = o.all || o.sources;
    let do_packages = o.all || o.packages;
    let do_build = o.all || o.build;

    if !do_pacman && !do_sources && !do_packages && !do_build {
        println!("Nothing selected. Use: --pkgcache --sources --packages --build | --all");
        return Ok(());
    }

    if do_pacman {
        clean_dir(Path::new(PACMAN_CACHE), "package cache", o)?;
    }
    for home in home_dirs() {
        for (on, sub) in [
            (do_sources, "sources"),
            (do_packages, "packages"),
            (do_build, "build"),
        ] {
            if !on {
                continue;
            }
            if let Some(dir) = safe_cache_dir(&home, sub) {
                clean_dir(&dir, &format!("{} {}", home.display(), sub), o)?;
            }
        }
    }
    Ok(())
}

/// `<home>/.cache/aura-emerge/<sub>` if every component is a real dir.
fn safe_cache_dir(home: &Path, sub: &str) -> Option<PathBuf> {
    let mut p = home.to_path_buf();
    for c in AE_USER_CACHE.split('/').chain(std::iter::once(sub)) {
        p.push(c);
        if !is_real_dir(&p) {
            return None;
        }
    }
    Some(p)
}

fn clean_dir(dir: &Path, label: &str, o: &Opts) -> Result<()> {
    if !is_real_dir(dir) {
        println!("{} {} — not present, skip", ">>>".yellow().bold(), label);
        return Ok(());
    }
    let count = fs::read_dir(dir)
        .map(|rd| rd.flatten().count())
        .unwrap_or(0);
    if count == 0 {
        println!("{} {} — empty", ">>>".green().bold(), label);
        return Ok(());
    }
    if o.pretend {
        println!(
            "{} {} — would delete {} item(s) in {}",
            ">>>".cyan().bold(),
            label,
            count,
            dir.display()
        );
        return Ok(());
    }
    if !o.yes
        && !confirm(&format!(
            "Delete {} item(s) under {}?",
            count,
            dir.display()
        ))
    {
        println!("skipped {}", label);
        return Ok(());
    }
    // Remove contents, keep the directory. file_type() does not follow
    // symlinks, and remove_dir_all/remove_file unlink links themselves.
    for e in fs::read_dir(dir).with_context(|| dir.display().to_string())? {
        let e = e?;
        let p = e.path();
        if e.file_type()?.is_dir() {
            fs::remove_dir_all(&p).with_context(|| p.display().to_string())?;
        } else {
            fs::remove_file(&p).with_context(|| p.display().to_string())?;
        }
    }
    println!("{} cleaned {}", ">>>".green().bold(), label);
    Ok(())
}

fn home_dirs() -> Vec<PathBuf> {
    let mut v = vec![PathBuf::from("/root")];
    if let Ok(rd) = fs::read_dir("/home") {
        for e in rd.flatten() {
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                v.push(e.path());
            }
        }
    }
    v.sort();
    v
}
