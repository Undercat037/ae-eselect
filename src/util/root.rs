//! Root / TTY helpers.

use std::io::{self, Write};

pub fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

pub fn require_root(action: &str) -> anyhow::Result<()> {
    if is_root() {
        return Ok(());
    }
    anyhow::bail!("{} requires root (run via sudo/doas)", action)
}

fn stdin_is_tty() -> bool {
    unsafe { libc::isatty(0) == 1 }
}

/// y/N prompt. Without a TTY the answer is always "no".
pub fn confirm(prompt: &str) -> bool {
    if !stdin_is_tty() {
        return false;
    }
    eprint!("{} [y/N] ", prompt);
    let _ = io::stderr().flush();
    let mut buf = String::new();
    if io::stdin().read_line(&mut buf).is_err() {
        return false;
    }
    matches!(buf.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}
