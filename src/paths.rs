//! Canonical paths for AE
//!
//! Profile tree: only `/var/db/portage/profiles`.
//! Repos managed by eselect live in one owned file inside the AE
//! `repos.conf/` directory, so AE (FEATURES="repos") picks them up.

use std::path::{Path, PathBuf};

pub const MAKE_PROFILE: &str = "/etc/portage/make.profile";
pub const PORTAGE_DIR: &str = "/etc/portage";
pub const WORLD: &str = "/etc/portage/world";
pub const MAKE_CONF: &str = "/etc/portage/make.conf";

/// AE reads this path as a file or as a directory of `*.conf`.
pub const REPOS_CONF: &str = "/etc/portage/repos.conf";

/// File owned exclusively by ae-eselect (inside `repos.conf/`).
pub const ESELECT_CONF: &str = "/etc/portage/repos.conf/eselect.conf";

/// Sole profile tree root.
pub const PROFILE_ROOT: &str = "/var/db/portage/profiles";

/// Repo metadata + optional key material.
pub const REPOS_DB: &str = "/var/db/portage/repos";

pub const KEYRING_DIR: &str = "/usr/share/pacman/keyrings";
pub const PACMAN_GNUPG: &str = "/etc/pacman.d/gnupg";

pub const PACMAN_CACHE: &str = "/var/cache/pacman/pkg";
/// Relative to a home dir (AE honours $XDG_CACHE_HOME per user; we only
/// know the default location).
pub const AE_USER_CACHE: &str = ".cache/aura-emerge";

pub fn make_profile_target() -> Option<PathBuf> {
    let link = Path::new(MAKE_PROFILE);
    match std::fs::read_link(link) {
        Ok(t) if t.is_absolute() => Some(t),
        Ok(t) => Some(link.parent().unwrap_or(Path::new(PORTAGE_DIR)).join(t)),
        Err(_) => None,
    }
}
