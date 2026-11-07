//! `eselect db` — lock / unlock / status of the alpm database lock.
//!
//! Pacman/libalpm uses `/var/lib/pacman/db.lck`. AE and pacman both
//! honour it; this module lets the admin clear a stale lock or take
//! an exclusive lock for maintenance.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use colored::Colorize;

use crate::util::root::require_root;

const DB_LCK: &str = "/var/lib/pacman/db.lck";

/// Process names that may legitimately hold the lock.
const HOLDERS: &[&str] = &["pacman", "emerge", "paru", "yay", "pamac-daemon"];

/// (pid, comm) of running package-manager processes.
fn running_holders() -> Vec<(u32, String)> {
    let me = std::process::id();
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir("/proc") else {
        return out;
    };
    for e in rd.flatten() {
        let Some(pid) = e.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        if pid == me {
            continue;
        }
        if let Ok(comm) = fs::read_to_string(e.path().join("comm")) {
            let comm = comm.trim();
            if HOLDERS.contains(&comm) {
                out.push((pid, comm.to_string()));
            }
        }
    }
    out
}

pub fn cmd_status() {
    let p = Path::new(DB_LCK);
    let procs = running_holders();
    if p.exists() {
        let age = fs::metadata(p)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.elapsed().ok())
            .map(|d| format!(" (age {}s)", d.as_secs()))
            .unwrap_or_default();
        println!("{} locked{}", "db".yellow().bold(), age);
        println!("  {}", DB_LCK);
        if procs.is_empty() {
            println!("  no pacman/emerge process running — lock looks stale");
        }
    } else {
        println!("{} unlocked", "db".green().bold());
    }
    for (pid, comm) in procs {
        println!("  running: {} (pid {})", comm, pid);
    }
}

pub fn cmd_lock() -> Result<()> {
    require_root("eselect db lock")?;
    let p = Path::new(DB_LCK);
    if p.exists() {
        bail!("already locked ({})", DB_LCK);
    }
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(p, format!("ae-eselect:{}\n", std::process::id()))
        .with_context(|| format!("create {}", DB_LCK))?;
    println!("{} locked {}", ">>>".green().bold(), DB_LCK);
    Ok(())
}

pub fn cmd_unlock(force: bool) -> Result<()> {
    require_root("eselect db unlock")?;
    let p = Path::new(DB_LCK);
    if !p.exists() {
        println!("{} already unlocked", ">>>".green().bold());
        return Ok(());
    }
    let procs = running_holders();
    if !force && !procs.is_empty() {
        let list: Vec<String> = procs.iter().map(|(p, c)| format!("{}({})", c, p)).collect();
        bail!(
            "refusing to remove the lock, still running: {} (use --force to override)",
            list.join(", ")
        );
    }
    fs::remove_file(p).with_context(|| format!("remove {}", DB_LCK))?;
    println!("{} unlocked {}", ">>>".green().bold(), DB_LCK);
    Ok(())
}
