//! `eselect profile` — list / set / show
//!
//! Tree root: `/var/db/portage/profiles` only.

use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use colored::Colorize;

use crate::paths::{self, MAKE_PROFILE, PROFILE_ROOT};
use crate::util::fs::{ensure_dir, relative_from};
use crate::util::root::{confirm, require_root};

/// Leaf profiles: (root, relative path from that root).
pub fn list_profiles() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let root = PROFILE_ROOT;
    let root_p = Path::new(root);
    if !root_p.is_dir() {
        return out;
    }
    let mut q = VecDeque::from([root_p.to_path_buf()]);
    while let Some(dir) = q.pop_front() {
        let kids = subdirs(&dir);
        if kids.is_empty() {
            if has_profile_files(&dir) {
                if let Ok(rel) = dir.strip_prefix(root_p) {
                    let s = rel.display().to_string();
                    if !s.is_empty() {
                        out.push((root.to_string(), s));
                    }
                }
            }
        } else {
            q.extend(kids);
        }
    }
    out.sort();
    out.dedup();
    out
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut v: Vec<PathBuf> = rd
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|s| !s.starts_with('.') && s != "files")
                .unwrap_or(false)
        })
        .collect();
    v.sort();
    v
}

fn has_profile_files(dir: &Path) -> bool {
    [
        "packages",
        "package.profile",
        "package.mask",
        "make.defaults",
        "parent",
    ]
    .iter()
    .any(|f| dir.join(f).is_file())
}

pub fn list_numbered() -> Vec<(usize, String, String, bool)> {
    let current =
        paths::make_profile_target().and_then(|p| std::fs::canonicalize(&p).ok().or(Some(p)));
    let mut out = Vec::new();
    for (i, (root, rel)) in list_profiles().into_iter().enumerate() {
        let full = PathBuf::from(&root).join(&rel);
        let full_c = std::fs::canonicalize(&full).unwrap_or_else(|_| full.clone());
        let is_cur = current.as_ref().map(|c| c == &full_c).unwrap_or(false);
        out.push((i + 1, root, rel, is_cur));
    }
    out
}

pub fn cmd_list() {
    let rows = list_numbered();
    if rows.is_empty() {
        println!("No profiles found under:\n  {}", PROFILE_ROOT);
        return;
    }
    println!("Available profile symlink targets:");
    for (n, _root, rel, cur) in &rows {
        let mark = if *cur { " * " } else { "   " };
        println!("{:>4}{}{}", n, mark, rel);
    }
    println!();
    if let Some(p) = paths::make_profile_target() {
        println!("Current {} symlink:", MAKE_PROFILE);
        println!("  -> {}", p.display());
    } else {
        println!("{} not set", MAKE_PROFILE);
    }
}

pub fn cmd_show() {
    match paths::make_profile_target() {
        Some(p) => {
            println!("Current {} symlink:", MAKE_PROFILE);
            if let Ok(rel) = p.strip_prefix(PROFILE_ROOT) {
                println!("  {}", rel.display());
            } else {
                println!("  {}", p.display());
            }
            println!("  -> {}", p.display());
        }
        None => println!("{} is not set", MAKE_PROFILE),
    }
}

pub fn cmd_set(spec: &str) -> Result<()> {
    require_root("eselect profile set")?;
    let target = resolve_spec(spec)?;
    if !target.is_dir() {
        bail!("not a profile directory: {}", target.display());
    }
    if !is_listed_leaf(&target) {
        bail!(
            "not a leaf profile of the tree: {} (try `eselect profile list`)",
            target.display()
        );
    }

    let link_path = Path::new(MAKE_PROFILE);
    let parent = link_path.parent().unwrap_or(Path::new(paths::PORTAGE_DIR));
    ensure_dir(parent)?;
    let link_val = relative_from(parent, &target).unwrap_or_else(|| target.clone());

    // A real dir/file here is user data: back it up, never delete it.
    if let Ok(md) = link_path.symlink_metadata() {
        if !md.file_type().is_symlink() {
            let bak = format!("{}.bak", MAKE_PROFILE);
            if Path::new(&bak).symlink_metadata().is_ok() {
                bail!(
                    "{} is not a symlink and {} already exists",
                    MAKE_PROFILE,
                    bak
                );
            }
            if !confirm(&format!(
                "{} is not a symlink. Move it to {}?",
                MAKE_PROFILE, bak
            )) {
                bail!("aborted");
            }
            std::fs::rename(link_path, &bak)
                .with_context(|| format!("cannot move {} -> {}", MAKE_PROFILE, bak))?;
            eprintln!(
                "{} moved {} -> {}",
                ">>>".yellow().bold(),
                MAKE_PROFILE,
                bak
            );
        }
    }

    // Atomic swap: build the new link next to the old one, then rename.
    let tmp = parent.join(".make.profile.new");
    let _ = std::fs::remove_file(&tmp);
    std::os::unix::fs::symlink(&link_val, &tmp)
        .with_context(|| format!("cannot create {} -> {}", tmp.display(), link_val.display()))?;
    std::fs::rename(&tmp, link_path)
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        })
        .with_context(|| format!("cannot replace {}", MAKE_PROFILE))?;

    println!(
        "{} {} -> {}",
        ">>>".green().bold(),
        MAKE_PROFILE,
        target.display()
    );
    Ok(())
}

fn is_listed_leaf(target: &Path) -> bool {
    let t = std::fs::canonicalize(target).unwrap_or_else(|_| target.to_path_buf());
    list_profiles().into_iter().any(|(root, rel)| {
        let full = PathBuf::from(root).join(rel);
        std::fs::canonicalize(&full).unwrap_or(full) == t
    })
}

fn resolve_spec(spec: &str) -> Result<PathBuf> {
    let spec = spec.trim();
    if let Ok(n) = spec.parse::<usize>() {
        return list_numbered()
            .into_iter()
            .find(|(i, _, _, _)| *i == n)
            .map(|(_, root, rel, _)| PathBuf::from(root).join(rel))
            .ok_or_else(|| anyhow::anyhow!("no profile number {} (try list)", n));
    }
    let p = PathBuf::from(spec);
    if p.is_dir() {
        return Ok(std::fs::canonicalize(&p).unwrap_or(p));
    }
    let cand = Path::new(PROFILE_ROOT).join(spec);
    if cand.is_dir() {
        return Ok(std::fs::canonicalize(&cand).unwrap_or(cand));
    }
    bail!("unknown profile '{}' (path or index from list)", spec)
}

#[allow(dead_code)]
pub fn expand_parents(leaf: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    expand_rec(leaf, &mut out, &mut seen, 0)?;
    Ok(out)
}

fn expand_rec(
    dir: &Path,
    out: &mut Vec<PathBuf>,
    seen: &mut HashSet<String>,
    depth: usize,
) -> Result<()> {
    if depth > 64 {
        bail!("profile parent depth > 64");
    }
    let key = dir.to_string_lossy().to_string();
    if !seen.insert(key) {
        bail!("cycle involving {}", dir.display());
    }
    let parent_file = dir.join("parent");
    if parent_file.is_file() {
        let text = std::fs::read_to_string(&parent_file)
            .with_context(|| parent_file.display().to_string())?;
        for line in text.lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            let p = if let Some((_repo, path)) = t.split_once(':') {
                Path::new(PROFILE_ROOT).join(path)
            } else {
                dir.join(t)
            };
            let p = std::fs::canonicalize(&p).unwrap_or(p);
            expand_rec(&p, out, seen, depth + 1)?;
        }
    }
    if !out.iter().any(|d| d == dir) {
        out.push(dir.to_path_buf());
    }
    Ok(())
}
