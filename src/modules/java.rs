//! `eselect java` — built-in replacement for Arch's `archlinux-java`.
//!
//! Layout (same as java-environment-common):
//!   /usr/lib/jvm/<env>/bin/java            full environment (JDK or JRE)
//!   /usr/lib/jvm/<env>/jre/bin/java        listed as `<env>/jre`
//!   /usr/lib/jvm/default                   symlink to the default env
//!   /usr/lib/jvm/default-runtime           symlink to the default runtime
//!
//! Also reachable as `archlinux-java <action>` (argv[0] dispatch in main),
//! so Arch java packages' install scripts keep working.

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::out;
use crate::util::root::require_root;

pub const JVM_DIR: &str = "/usr/lib/jvm";
const DEFAULT: &str = "default";
const DEFAULT_RUNTIME: &str = "default-runtime";

fn is_exec(p: &Path) -> bool {
    fs::metadata(p)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Sorted names of valid environments under `dir`.
pub fn list_envs(dir: &Path) -> Vec<String> {
    let mut v = Vec::new();
    let Ok(rd) = fs::read_dir(dir) else {
        return v;
    };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name == DEFAULT || name == DEFAULT_RUNTIME || name.starts_with('.') {
            continue;
        }
        let p = e.path();
        // Skip symlinks other than real env dirs (e.g. compat aliases)
        // only when they dangle; a link to a valid env is still an env.
        if is_exec(&p.join("bin/java")) {
            v.push(name.clone());
        }
        if is_exec(&p.join("jre/bin/java")) {
            v.push(format!("{}/jre", name));
        }
    }
    v.sort();
    v
}

/// Name the `default` symlink points to (relative to `dir`), if valid.
pub fn current_default(dir: &Path) -> Option<String> {
    link_name(dir, DEFAULT)
}

fn link_name(dir: &Path, link: &str) -> Option<String> {
    let t = fs::read_link(dir.join(link)).ok()?;
    let t = if t.is_absolute() {
        t.strip_prefix(dir).ok()?.to_path_buf()
    } else {
        t
    };
    Some(t.to_string_lossy().trim_end_matches('/').to_string())
}

fn set_link(dir: &Path, link: &str, target: &str) -> Result<()> {
    let path = dir.join(link);
    let tmp = dir.join(format!(".{}.new", link));
    let _ = fs::remove_file(&tmp);
    symlink(target, &tmp).with_context(|| format!("cannot create {}", tmp.display()))?;
    fs::rename(&tmp, &path)
        .inspect_err(|_| {
            let _ = fs::remove_file(&tmp);
        })
        .with_context(|| format!("cannot replace {}", path.display()))
}

fn valid_env_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('/')
        && !name
            .split('/')
            .any(|c| c.is_empty() || c == "." || c == "..")
}

/// Highest `java-<N>…` wins; ties and non-numeric names fall back to
/// alphabetical order. Full JDK/JRE dirs are preferred over `<env>/jre`.
fn pick_best(envs: &[String]) -> Option<String> {
    fn major(n: &str) -> u32 {
        n.strip_prefix("java-")
            .and_then(|r| {
                let d: String = r.chars().take_while(|c| c.is_ascii_digit()).collect();
                d.parse().ok()
            })
            .unwrap_or(0)
    }
    envs.iter()
        .max_by(|a, b| {
            let ka = (!a.ends_with("/jre"), major(a));
            let kb = (!b.ends_with("/jre"), major(b));
            ka.cmp(&kb).then_with(|| b.cmp(a))
        })
        .cloned()
}

pub fn cmd_status() -> Result<()> {
    status_in(Path::new(JVM_DIR))
}

fn status_in(dir: &Path) -> Result<()> {
    let envs = list_envs(dir);
    let def = current_default(dir);
    out::write_list_start("Available Java environments:");
    if envs.is_empty() {
        out::write_none_found("(none found)");
        return Ok(());
    }
    for e in &envs {
        if def.as_deref() == Some(e.as_str()) {
            println!("  {} (default)", e);
        } else {
            println!("  {}", e);
        }
    }
    Ok(())
}

pub fn cmd_get() -> Result<()> {
    match current_default(Path::new(JVM_DIR)) {
        Some(d) => {
            println!("{}", d);
            Ok(())
        }
        None => bail!("no Java environment is set as default"),
    }
}

pub fn cmd_set(env: &str) -> Result<()> {
    require_root("eselect java set")?;
    set_in(Path::new(JVM_DIR), env)
}

fn set_in(dir: &Path, env: &str) -> Result<()> {
    if !valid_env_name(env) {
        bail!("'{}' is not a valid Java environment name", env);
    }
    let bin: PathBuf = dir.join(env).join("bin/java");
    if !is_exec(&bin) {
        bail!(
            "'{}' is not a valid Java environment (no {}); see `eselect java status`",
            env,
            bin.display()
        );
    }
    set_link(dir, DEFAULT, env)?;
    // The runtime link follows the default env.
    set_link(dir, DEFAULT_RUNTIME, env)?;
    Ok(())
}

pub fn cmd_unset() -> Result<()> {
    require_root("eselect java unset")?;
    unset_in(Path::new(JVM_DIR))
}

fn unset_in(dir: &Path) -> Result<()> {
    for l in [DEFAULT, DEFAULT_RUNTIME] {
        let p = dir.join(l);
        if p.symlink_metadata().is_ok() {
            fs::remove_file(&p).with_context(|| format!("remove {}", p.display()))?;
        }
    }
    Ok(())
}

/// Quiet on success: package install scripts call this on every java
/// install/remove.
pub fn cmd_fix() -> Result<()> {
    require_root("eselect java fix")?;
    fix_in(Path::new(JVM_DIR))
}

fn fix_in(dir: &Path) -> Result<()> {
    let envs = list_envs(dir);
    if let Some(d) = current_default(dir) {
        if envs.contains(&d) {
            return Ok(()); // default is valid
        }
    }
    match pick_best(&envs) {
        Some(best) => set_in(dir, &best),
        None => {
            // Nothing usable left: drop dangling links.
            unset_in(dir)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_env(dir: &Path, name: &str, jre: bool) {
        let b = if jre {
            dir.join(name).join("jre/bin")
        } else {
            dir.join(name).join("bin")
        };
        fs::create_dir_all(&b).unwrap();
        let j = b.join("java");
        fs::write(&j, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&j, fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ae-java-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn lists_and_sets() {
        let d = tmp("set");
        mk_env(&d, "java-17-openjdk", false);
        mk_env(&d, "java-8-openjdk", false);
        mk_env(&d, "java-8-openjdk", true);
        let l = list_envs(&d);
        assert_eq!(
            l,
            vec!["java-17-openjdk", "java-8-openjdk", "java-8-openjdk/jre"]
        );
        set_in(&d, "java-8-openjdk/jre").unwrap();
        assert_eq!(current_default(&d).as_deref(), Some("java-8-openjdk/jre"));
        assert!(set_in(&d, "../etc").is_err());
        assert!(set_in(&d, "nope").is_err());
        unset_in(&d).unwrap();
        assert!(current_default(&d).is_none());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn fix_picks_newest_and_repairs() {
        let d = tmp("fix");
        mk_env(&d, "java-11-openjdk", false);
        mk_env(&d, "java-21-openjdk", false);
        fix_in(&d).unwrap();
        assert_eq!(current_default(&d).as_deref(), Some("java-21-openjdk"));
        // dangling default is repaired
        set_link(&d, DEFAULT, "java-99-gone").unwrap();
        fix_in(&d).unwrap();
        assert_eq!(current_default(&d).as_deref(), Some("java-21-openjdk"));
        // valid default is kept
        set_in(&d, "java-11-openjdk").unwrap();
        fix_in(&d).unwrap();
        assert_eq!(current_default(&d).as_deref(), Some("java-11-openjdk"));
        let _ = fs::remove_dir_all(&d);
    }
}
