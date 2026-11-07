//! `eselect news` — thin wrapper around `emerge --news`.
//!
//! When emerge is present, delegates. Otherwise prints a short message.

use std::process::Command;

use anyhow::{Result, bail};

pub fn cmd_list() -> Result<()> {
    run_emerge(&["--news"])
}

pub fn cmd_read(n: Option<&str>) -> Result<()> {
    match n {
        Some("all") => run_emerge(&["--news", "all"]),
        Some(x) => run_emerge(&["--news", x]),
        None => run_emerge(&["--news"]),
    }
}

fn run_emerge(args: &[&str]) -> Result<()> {
    let status = Command::new("emerge").args(args).status();
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => bail!("emerge exited with {}", s),
        Err(e) => bail!("cannot run emerge {}: is aura-emerge installed?", e),
    }
}
