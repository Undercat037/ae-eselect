//! Output helpers modelled on Gentoo eselect's `libs/output.bash`.
//!
//! Colours (as in eselect): header = bold green, list left column = bold,
//! highlight / marker = bold blue, warning = bold yellow, error = bold red.
//! `--brief` drops list headers and the `*` marker.

#![allow(dead_code)]

use std::sync::atomic::{AtomicBool, Ordering};

use colored::Colorize;

static BRIEF: AtomicBool = AtomicBool::new(false);

pub fn set_brief(on: bool) {
    BRIEF.store(on, Ordering::Relaxed);
}

pub fn is_brief() -> bool {
    BRIEF.load(Ordering::Relaxed)
}

/// `!!! Error: …` on stderr.
pub fn write_error_msg(msg: &str) {
    eprintln!("{} {}", "!!! Error:".red().bold(), msg);
}

/// `!!! Warning: …` on stderr.
pub fn write_warning_msg(msg: &str) {
    eprintln!("{} {}", "!!! Warning:".yellow().bold(), msg);
}

/// List header ("Available profile symlink targets:"). Hidden by `--brief`.
pub fn write_list_start(text: &str) {
    if is_brief() {
        return;
    }
    println!("{}", text.green().bold());
}

/// Section heading (help screens). Hidden by `--brief`, like eselect does.
pub fn write_heading(text: &str) {
    if is_brief() {
        return;
    }
    println!("{}", text.green().bold());
}

/// `  key<pad>value`, value column at 28 (eselect's kv layout).
/// A key that does not fit puts the value on the next line.
pub fn write_kv_list_entry_at(indent: usize, key: &str, value: &str) {
    const VALUE_COL: usize = 28;
    let pad_to = VALUE_COL.saturating_sub(indent);
    let shown = key.bold().to_string();
    let spaces = " ".repeat(indent);
    if value.is_empty() {
        println!("{}{}", spaces, shown);
    } else if key.chars().count() + 1 > pad_to {
        println!("{}{}", spaces, shown);
        println!("{}{}", " ".repeat(VALUE_COL), value);
    } else {
        let fill = " ".repeat(pad_to - key.chars().count());
        println!("{}{}{}{}", spaces, shown, fill, value);
    }
}

pub fn write_kv_list_entry(key: &str, value: &str) {
    write_kv_list_entry_at(2, key, value);
}

/// `  [N]   item` — the `[N]` column is padded to 6 like eselect does.
pub fn write_numbered_list_entry(n: usize, item: &str) {
    let key = format!("[{}]", n);
    let pad = " ".repeat(6usize.saturating_sub(key.chars().count()).max(1));
    println!("  {}{}{}", key.bold(), pad, item);
}

/// Mark an entry as active: `item *` (no marker in brief mode).
pub fn highlight_marker(item: &str) -> String {
    if is_brief() {
        item.to_string()
    } else {
        format!("{} {}", item, "*".blue().bold())
    }
}

/// Empty-list message, same look as `write_numbered_list -m`.
pub fn write_none_found(msg: &str) {
    println!("  {}", msg.bold());
}
