//! `eselect repository` — list / enable / disable / remove / add
//!
//! Repos managed by eselect live in one file that AE reads as part of
//! `repos.conf/` (FEATURES="repos"):
//!
//!   /etc/portage/repos.conf/eselect.conf
//!
//! eselect owns this file. Disabled repos stay in it as `#| `-prefixed
//! blocks (AE sees them as comments), so `enable` can restore them.
//! Other files in repos.conf/ are never modified.
//!
//! Key import (on enable/add):
//! 1. /usr/share/pacman/keyrings/<name>.*
//! 2. /var/db/portage/repos/<name>/keys/

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use colored::Colorize;

use crate::out;
use crate::paths::{ESELECT_CONF, KEYRING_DIR, MAKE_CONF, PACMAN_GNUPG, REPOS_CONF, REPOS_DB};
use crate::util::fs::{atomic_write, ensure_dir};
use crate::util::pkgdb;
use crate::util::root::{confirm, require_root};

use std::os::unix::process::CommandExt;

/// Keys AE's repos.conf parser understands; anything else is dropped.
const KNOWN_KEYS: &[&str] = &[
    "priority",
    "sync-uri",
    "include",
    "verify-signature",
    "location",
    "auto-sync",
    "sync-type",
];

const DISABLED_PREFIX: &str = "#| ";

#[derive(Debug, Clone, Default)]
pub struct RepoEntry {
    pub name: String,
    /// Ordered `key = value` pairs (keys lower-case, `_` -> `-`).
    pub keys: Vec<(String, String)>,
    pub enabled: bool,
}

impl RepoEntry {
    fn get(&self, key: &str) -> Option<&str> {
        self.keys
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    fn set(&mut self, key: &str, val: &str) {
        match self.keys.iter_mut().find(|(k, _)| k == key) {
            Some(kv) => kv.1 = val.to_string(),
            None => self.keys.push((key.to_string(), val.to_string())),
        }
    }

    fn has_source(&self) -> bool {
        self.get("sync-uri").is_some() || self.get("include").is_some()
    }
}

/// Parse repos.conf-style text. `#| ` lines are read as a disabled block.
fn parse_conf(text: &str) -> Vec<RepoEntry> {
    let mut out: Vec<RepoEntry> = Vec::new();
    let mut cur: Option<RepoEntry> = None;
    for raw in text.lines() {
        let t = raw.trim();
        let (enabled, line) = match t.strip_prefix("#|") {
            Some(rest) => (false, rest.trim()),
            None => (true, t),
        };
        if line.is_empty() || (enabled && line.starts_with('#')) {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            if let Some(e) = cur.take() {
                out.push(e);
            }
            cur = Some(RepoEntry {
                name: name.trim().to_string(),
                keys: Vec::new(),
                enabled,
            });
            continue;
        }
        let Some(ref mut e) = cur else { continue };
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim().to_ascii_lowercase().replace('_', "-");
            let k = if k == "siglevel" || k == "verify-signature" {
                "verify-signature".to_string()
            } else {
                k
            };
            e.keys.push((k, v.trim().to_string()));
        }
    }
    if let Some(e) = cur {
        out.push(e);
    }
    out
}

/// Load entries from the eselect-owned conf only.
pub fn load_eselect_repos() -> Result<Vec<RepoEntry>> {
    let path = Path::new(ESELECT_CONF);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    Ok(parse_conf(&text))
}

/// `[name]` sections defined in repos.conf outside of eselect.conf.
fn external_repo_names() -> Vec<String> {
    let root = Path::new(REPOS_CONF);
    let mut files: Vec<PathBuf> = Vec::new();
    match fs::symlink_metadata(root) {
        Ok(m) if m.is_file() => files.push(root.to_path_buf()),
        Ok(m) if m.is_dir() => {
            if let Ok(rd) = fs::read_dir(root) {
                for e in rd.flatten() {
                    let p = e.path();
                    if p.extension().and_then(|x| x.to_str()) == Some("conf")
                        && p != Path::new(ESELECT_CONF)
                    {
                        files.push(p);
                    }
                }
            }
        }
        _ => {}
    }
    let mut names = Vec::new();
    for f in files {
        if let Ok(t) = fs::read_to_string(&f) {
            names.extend(
                parse_conf(&t)
                    .into_iter()
                    .filter(|e| e.enabled)
                    .map(|e| e.name),
            );
        }
    }
    names
}

/// repos.conf must be a real directory for eselect.conf to live in it.
/// A single-file layout is converted (with confirmation).
fn ensure_repos_dir() -> Result<()> {
    let root = Path::new(REPOS_CONF);
    match fs::symlink_metadata(root) {
        Err(_) => {
            ensure_dir(root)?;
            Ok(())
        }
        Ok(m) if m.file_type().is_symlink() => {
            bail!(
                "{} is a symlink; AE refuses symlinked repos.conf",
                REPOS_CONF
            )
        }
        Ok(m) if m.is_dir() => Ok(()),
        Ok(_) => {
            let moved = root.join("00-main.conf");
            if !confirm(&format!(
                "{} is a single file. Convert to a directory (old file -> {})?",
                REPOS_CONF,
                moved.display()
            )) {
                bail!("{} must be a directory for eselect (aborted)", REPOS_CONF);
            }
            let tmp = root.with_file_name("repos.conf.migrate");
            fs::rename(root, &tmp).context("move repos.conf aside")?;
            fs::create_dir(root).context("create repos.conf/")?;
            fs::rename(&tmp, &moved).context("move old repos.conf into repos.conf/")?;
            println!(
                "{} {} -> {}",
                ">>>".green().bold(),
                REPOS_CONF,
                moved.display()
            );
            Ok(())
        }
    }
}

/// Rewrite eselect.conf from the full entry list.
fn write_eselect_conf(entries: &[RepoEntry]) -> Result<()> {
    ensure_repos_dir()?;
    let path = Path::new(ESELECT_CONF);

    let mut body = String::from(
        "# Managed by eselect (ae-eselect). Use: eselect repository enable|disable|add|remove\n\
         # Read by emerge with FEATURES=\"repos\". Blocks starting with \"#| \" are disabled.\n\n",
    );
    for e in entries {
        let p = if e.enabled { "" } else { DISABLED_PREFIX };
        body.push_str(&format!("{}[{}]\n", p, e.name));
        for (k, v) in &e.keys {
            body.push_str(&format!("{}{} = {}\n", p, k, v));
        }
        body.push('\n');
    }
    atomic_write(path, &body).with_context(|| format!("write {}", path.display()))
}

/// Repo template from `/var/db/portage/repos/<name>/…`, filtered to keys
/// AE understands.
fn entry_from_template(name: &str) -> RepoEntry {
    let mut e = RepoEntry {
        name: name.to_string(),
        enabled: true,
        ..Default::default()
    };
    for cand in [
        Path::new(REPOS_DB)
            .join(name)
            .join(format!("{}.conf", name)),
        Path::new(REPOS_DB).join(name).join("conf"),
    ] {
        let Ok(text) = fs::read_to_string(&cand) else {
            continue;
        };
        let parsed = parse_conf(&text);
        let found = parsed
            .iter()
            .position(|x| x.name == name)
            .or(if parsed.is_empty() { None } else { Some(0) });
        if let Some(i) = found {
            for (k, v) in &parsed[i].keys {
                if KNOWN_KEYS.contains(&k.as_str()) {
                    e.set(k, v);
                }
            }
        }
        break;
    }
    e
}

fn features_has_repos() -> bool {
    let Ok(text) = fs::read_to_string(MAKE_CONF) else {
        return false;
    };
    let mut on = false;
    for l in text.lines() {
        let l = l.trim();
        if let Some(v) = l.strip_prefix("FEATURES=") {
            on = v
                .split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_'))
                .any(|t| t == "repos");
        }
    }
    on
}

fn hint_features() {
    if !features_has_repos() {
        out::write_warning_msg(&format!(
            "FEATURES=\"repos\" is not set in {} — emerge ignores repos.conf until it is",
            MAKE_CONF
        ));
    }
}

/// Warn if `name` is also defined elsewhere in repos.conf; ask to go on.
fn confirm_not_duplicate(name: &str) -> bool {
    if external_repo_names().iter().any(|n| n == name) {
        eprintln!(
            "{} [{}] is already defined in {} (outside eselect.conf)",
            ">>>".yellow().bold(),
            name,
            REPOS_CONF
        );
        return confirm("Add it to eselect.conf anyway?");
    }
    true
}

pub fn cmd_list(installed_only: bool) -> Result<()> {
    let managed = load_eselect_repos()?;
    let external = external_repo_names();
    // (name, mark)
    let mut rows: Vec<(String, char)> = Vec::new();

    for e in &managed {
        rows.push((e.name.clone(), if e.enabled { '*' } else { '#' }));
    }
    for n in &external {
        if !rows.iter().any(|(k, _)| k == n) {
            rows.push((n.clone(), '+'));
        }
    }

    if !installed_only {
        let mut known: Vec<String> = Vec::new();
        if let Ok(rd) = fs::read_dir(KEYRING_DIR) {
            for ent in rd.flatten() {
                let name = ent.file_name().to_string_lossy().to_string();
                if let Some(n) = name.strip_suffix(".gpg") {
                    known.push(n.to_string());
                }
            }
        }
        if let Ok(rd) = fs::read_dir(REPOS_DB) {
            for ent in rd.flatten() {
                if ent.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    known.push(ent.file_name().to_string_lossy().to_string());
                }
            }
        }
        for n in known {
            if !rows.iter().any(|(k, _)| *k == n) {
                rows.push((n, ' '));
            }
        }
    }

    rows.sort();
    out::write_list_start("Available repositories:");
    if rows.is_empty() {
        out::write_none_found("(none found)");
        return Ok(());
    }
    for (i, (name, mark)) in rows.iter().enumerate() {
        // '*' enabled in eselect.conf, '#' disabled, '+' defined in another
        // repos.conf file, ' ' known (keys/template present) but not enabled
        let label = match mark {
            '*' => out::highlight_marker(name),
            '#' => format!("{} {}", name, "(disabled)".yellow()),
            '+' => format!("{} {}", name, "(repos.conf)".cyan()),
            _ => name.clone(),
        };
        out::write_numbered_list_entry(i + 1, &label);
    }
    Ok(())
}

pub fn cmd_enable(names: &[String]) -> Result<()> {
    require_root("eselect repository enable")?;
    let mut entries = load_eselect_repos()?;
    let mut changed = false;

    for name in names {
        if let Some(e) = entries.iter_mut().find(|e| e.name == *name) {
            if e.enabled {
                println!("{} {} already enabled", ">>>".green().bold(), name);
                continue;
            }
            e.enabled = true;
            println!("{} re-enabled {}", ">>>".green().bold(), name);
        } else {
            if !confirm_not_duplicate(name) {
                println!("skipped {}", name);
                continue;
            }
            let e = entry_from_template(name);
            if !e.has_source() {
                out::write_error_msg(&format!(
                    "no template for '{}' — add it with: eselect repository add {} <sync-uri>",
                    name, name
                ));
                continue;
            }
            println!("{} add {} -> {}", ">>>".green().bold(), name, ESELECT_CONF);
            entries.push(e);
        }
        changed = true;
        match import_keys(name) {
            Ok(msg) => println!("{} {}", ">>>".green().bold(), msg),
            Err(err) => eprintln!("{} keys for {}: {}", ">>>".yellow().bold(), name, err),
        }
    }

    if changed {
        write_eselect_conf(&entries)?;
        hint_features();
    }
    Ok(())
}

pub fn cmd_disable(names: &[String]) -> Result<()> {
    require_root("eselect repository disable")?;
    let mut entries = load_eselect_repos()?;
    let mut changed = false;
    for name in names {
        match entries.iter_mut().find(|e| e.name == *name) {
            Some(e) if e.enabled => {
                e.enabled = false;
                changed = true;
                println!("{} disabled {}", ">>>".green().bold(), name);
            }
            Some(_) => println!("{} {} already disabled", ">>>".yellow().bold(), name),
            None => eprintln!(
                "{} '{}' not in {}",
                ">>>".yellow().bold(),
                name,
                ESELECT_CONF
            ),
        }
    }
    if changed {
        write_eselect_conf(&entries)?;
    }
    Ok(())
}

pub fn cmd_remove(names: &[String], force: bool) -> Result<()> {
    require_root("eselect repository remove")?;
    let mut entries = load_eselect_repos()?;
    let before = entries.len();
    for name in names {
        if !entries.iter().any(|e| e.name == *name) {
            eprintln!(
                "{} '{}' not in {}",
                ">>>".yellow().bold(),
                name,
                ESELECT_CONF
            );
            continue;
        }
        if !force && !confirm(&format!("Remove '{}' from {}?", name, ESELECT_CONF)) {
            println!("skipped {}", name);
            continue;
        }
        entries.retain(|e| e.name != *name);
        println!("{} removed {}", ">>>".green().bold(), name);
    }
    if entries.len() != before {
        write_eselect_conf(&entries)?;
    }
    Ok(())
}

fn name_ok(n: &str) -> bool {
    !n.is_empty()
        && !n.starts_with('-')
        && n.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+'))
}

pub fn cmd_add(name: &str, sync_uri: &str, priority: i32, sig: Option<&str>) -> Result<()> {
    require_root("eselect repository add")?;
    if !name_ok(name) {
        bail!("invalid repository name '{}'", name);
    }
    if sync_uri.contains(char::is_whitespace) {
        bail!("invalid sync-uri '{}'", sync_uri);
    }
    let mut entries = load_eselect_repos()?;
    if entries.iter().any(|e| e.name == name) {
        bail!("'{}' already present in {}", name, ESELECT_CONF);
    }
    if !confirm_not_duplicate(name) {
        bail!("aborted");
    }
    let mut e = RepoEntry {
        name: name.to_string(),
        enabled: true,
        ..Default::default()
    };
    e.set("priority", &priority.to_string());
    e.set("sync-uri", sync_uri);
    if let Some(s) = sig {
        e.set("verify-signature", s);
    }
    entries.push(e);
    write_eselect_conf(&entries)?;
    println!(
        "{} {} added to {}",
        ">>>".green().bold(),
        name,
        ESELECT_CONF
    );
    match import_keys(name) {
        Ok(msg) => println!("{} {}", ">>>".green().bold(), msg),
        Err(err) => eprintln!("{} keys: {}", ">>>".yellow().bold(), err),
    }
    hint_features();
    Ok(())
}

// ── key import ──────────────────────────────────────────────────────────

pub fn import_keys(name: &str) -> Result<String> {
    let gpg = Path::new(KEYRING_DIR).join(format!("{}.gpg", name));
    if gpg.is_file() {
        populate_packaged(name)?;
        return Ok(format!("keyring '{}' populated from {}", name, KEYRING_DIR));
    }
    let keys_dir = Path::new(REPOS_DB).join(name).join("keys");
    let local_gpg = keys_dir.join(format!("{}.gpg", name));
    if local_gpg.is_file() {
        populate_from_dir(name, &keys_dir)?;
        return Ok(format!("keys imported from {}", keys_dir.display()));
    }
    if keys_dir.is_dir() {
        let mut any = false;
        for e in fs::read_dir(&keys_dir)?.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) == Some("gpg") {
                gpg_run(&["--import", p.to_str().unwrap()], None)?;
                any = true;
            }
        }
        if any {
            let trusted = keys_dir.join(format!("{}-trusted", name));
            if trusted.is_file() {
                apply_trusted(&trusted)?;
            }
            let _ = gpg_run(&["--check-trustdb"], None);
            return Ok(format!("keys imported from {}", keys_dir.display()));
        }
    }
    bail!(
        "no key material for '{}' in {} or {}",
        name,
        KEYRING_DIR,
        keys_dir.display()
    )
}

fn populate_packaged(name: &str) -> Result<()> {
    ensure_master_key()?;
    let base = Path::new(KEYRING_DIR).join(name);
    gpg_run(&["--import", &format!("{}.gpg", base.display())], None)?;
    let trusted = format!("{}-trusted", base.display());
    if Path::new(&trusted).is_file() {
        apply_trusted(Path::new(&trusted))?;
    }
    let revoked = format!("{}-revoked", base.display());
    if Path::new(&revoked).is_file() {
        apply_revoked(Path::new(&revoked))?;
    }
    gpg_run(&["--check-trustdb"], None)?;
    Ok(())
}

fn populate_from_dir(name: &str, keys_dir: &Path) -> Result<()> {
    ensure_master_key()?;
    let gpg = keys_dir.join(format!("{}.gpg", name));
    gpg_run(&["--import", gpg.to_str().unwrap()], None)?;
    let trusted = keys_dir.join(format!("{}-trusted", name));
    if trusted.is_file() {
        apply_trusted(&trusted)?;
    }
    let revoked = keys_dir.join(format!("{}-revoked", name));
    if revoked.is_file() {
        apply_revoked(&revoked)?;
    }
    gpg_run(&["--check-trustdb"], None)?;
    Ok(())
}

fn apply_trusted(path: &Path) -> Result<()> {
    let text = fs::read_to_string(path)?;
    let mut ownertrust = String::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let id = line.split(':').next().unwrap_or("");
        if !is_fpr(id) {
            continue;
        }
        let _ = gpg_run(&["--yes", "--lsign-key", id], None);
        ownertrust.push_str(line);
        if !line.contains(':') {
            ownertrust.push_str(":4:");
        }
        ownertrust.push('\n');
    }
    if !ownertrust.is_empty() {
        gpg_run(&["--import-ownertrust"], Some(&ownertrust))?;
    }
    Ok(())
}

fn apply_revoked(path: &Path) -> Result<()> {
    let text = fs::read_to_string(path)?;
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') || !is_fpr(line) {
            continue;
        }
        let _ = gpg_run(
            &["--command-fd", "0", "--edit-key", line],
            Some("disable\nsave\n"),
        );
    }
    Ok(())
}

fn is_fpr(s: &str) -> bool {
    (16..=40).contains(&s.len()) && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn ensure_master_key() -> Result<()> {
    let out = gpg_run(&["--list-secret-keys", "--with-colons"], None)?;
    if !out.lines().any(|l| l.starts_with("sec:")) {
        bail!(
            "alpm keyring not initialised — run `pacman-key --init` (or initialise /etc/pacman.d/gnupg) first"
        );
    }
    Ok(())
}

fn gpg_run(args: &[&str], input: Option<&str>) -> Result<String> {
    let mut cmd = Command::new("/usr/bin/gpg");
    cmd.args([
        "--homedir",
        PACMAN_GNUPG,
        "--no-permission-warning",
        "--quiet",
        "--batch",
    ])
    .args(args)
    .env("LC_ALL", "C")
    .stdin(if input.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    })
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
    unsafe {
        cmd.pre_exec(|| {
            libc::umask(0o022);
            Ok(())
        });
    }
    let mut child = cmd.spawn().context("spawn gpg")?;
    if let (Some(text), Some(mut stdin)) = (input, child.stdin.take()) {
        let _ = stdin.write_all(text.as_bytes());
    }
    let out = child.wait_with_output()?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
    }
    let err = String::from_utf8_lossy(&out.stderr);
    let last = err
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("failed");
    bail!("gpg {}: {}", args.first().unwrap_or(&"?"), last.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_enabled_and_disabled_blocks() {
        let t = "# c\n[a]\npriority = 50\nsync-uri = https://x/$repo\n\n#| [b]\n#| priority = 40\n#| SigLevel = Never\n";
        let v = parse_conf(t);
        assert_eq!(v.len(), 2);
        assert!(v[0].enabled && v[0].get("priority") == Some("50"));
        assert!(!v[1].enabled && v[1].name == "b");
        assert_eq!(v[1].get("verify-signature"), Some("Never"));
    }

    #[test]
    fn disabled_block_is_comment_for_ae() {
        // AE strips everything after '#', so "#| [b]" must yield nothing.
        let line = "#| [b]";
        assert_eq!(line.split('#').next().unwrap().trim(), "");
    }

    #[test]
    fn has_source_needs_uri_or_include() {
        let mut e = RepoEntry::default();
        assert!(!e.has_source());
        e.set("include", "/etc/pacman.d/mirrorlist");
        assert!(e.has_source());
    }
}

/// Print every package of one synced repository, one per line, straight
/// from `/var/lib/pacman/sync/<repo>.db` (no pacman/emerge involved).
/// Plain text only, so `> file` / `| tee file` produce a clean package list
/// for profiles.
pub fn cmd_packages(repo: &str, versions: bool, prefix: bool) -> Result<()> {
    let pkgs = pkgdb::read_sync(repo).map_err(|e| {
        let known = pkgdb::sync_repos();
        if known.is_empty() {
            e
        } else {
            e.context(format!("synced repositories: {}", known.join(", ")))
        }
    })?;
    let stdout = std::io::stdout();
    let mut w = std::io::BufWriter::new(stdout.lock());
    for p in &pkgs {
        let name = if prefix {
            format!("{}/{}", repo, p.name)
        } else {
            p.name.clone()
        };
        let line = if versions {
            format!("{} {}", name, p.version)
        } else {
            name
        };
        // A closed pipe (`| head`) is not an error.
        if writeln!(w, "{}", line).is_err() {
            return Ok(());
        }
    }
    let _ = w.flush();
    Ok(())
}
