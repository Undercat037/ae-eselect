<div align="center">
  <img src="misc/logo/ae-eselect_banner_386x140.svg" width="350" alt="ae-eselect logo">

# ae-eselect — eselect module for aura-emerge

[![AE_Site](https://img.shields.io/badge/AE_Site-333333?style=flat-square&logo=bookstack&logoColor=FFFFFF)](https://undercat037.github.io/aura-emerge/)
[![GitHub](https://img.shields.io/badge/GitHub-333333?style=flat-square&logo=github&logoColor=FFFFFF)](https://github.com/Undercat037/ae-eselect)
[![GitLab](https://img.shields.io/badge/GitLab-333333?style=flat-square&logo=gitlab&logoColor=FC6D26)](https://gitlab.com/Undercat037/ae-eselect)

en | [ua](UA-README.MD) | [ru](RU-README.MD)
</div>

> A modular configuration tool in the style of Gentoo `eselect` for **aura-emerge**: select profile, repositories and their keys, the world file, caches, the package database lock and Java environments - with Portage-like output.

As in Gentoo, `eselect` is a separate package, not part of `emerge`. Rather than a full port of the original, it serves as an auxiliary multitool for aura-emerge.

**Sections:** [Installation](#installation) · [Usage examples](#usage-examples) · [Modules](#modules) · [Repositories](#repositories) · [Files](#files)

## Installation

With aura-emerge (recommended):
```bash
git clone https://github.com/Undercat037/ae-eselect
cd ae-eselect
emerge --usepkgbuild ./ --ask
```

With makepkg:
```bash
git clone https://github.com/Undercat037/ae-eselect
cd ae-eselect
makepkg -si
```

With make:
```bash
git clone https://github.com/Undercat037/ae-eselect
cd ae-eselect
make install
```

<details>
<summary><h3>Manually</h3></summary>

```bash
git clone https://github.com/Undercat037/ae-eselect
cd ae-eselect
cargo build --release
sudo install -Dm755 target/release/eselect /usr/bin/eselect
eselect --gen-completions bash | sudo tee /usr/share/bash-completion/completions/eselect >/dev/null
eselect --gen-completions zsh  | sudo tee /usr/share/zsh/site-functions/_eselect >/dev/null
eselect --gen-completions fish | sudo tee /usr/share/fish/vendor_completions.d/eselect.fish >/dev/null
eselect --gen-manpage man && sudo install -Dm644 man/*.1 -t /usr/share/man/man1/
```

</details>

> **Dependencies:** `gnupg` `pacman` (libalpm)  
> **Optional:** `aura-emerge`

## Usage examples

```bash
# Overview
eselect                                  # global options and the module list
eselect profile                          # actions of one module
eselect profile help                     # the same, with descriptions

# Profile
eselect profile list                     # numbered list, the current one is marked *
eselect profile show                     # current make.profile link
sudo eselect profile set 3               # by number from `list`
sudo eselect profile set default/linux/amd64/23.0   # or by path

# Repositories
eselect repository list                  # configured and known repositories
eselect repository list -i               # configured only
sudo eselect repository enable chaotic-aur   # from the local database + key import
sudo eselect repository add myrepo 'https://example.org/$repo/os/$arch' -p 60
sudo eselect repository disable myrepo   # the block stays, enable restores it
sudo eselect repository remove -f myrepo

# Package list of a repository (plain text, safe to redirect into a file)
eselect repository packages core > packages
eselect repository packages -r -v extra | tee extra.list   # extra/foo 1.2-1

# Java (built-in archlinux-java)
eselect java status                      # environments, the current one is marked (default)
sudo eselect java set java-21-openjdk
sudo eselect java fix                    # repair the default link

# World
eselect world show
eselect world regen -p                   # what would be added
sudo eselect world regen -a              # add the missing ones, with confirmation

# News
eselect news                             # Arch News
eselect news read 8                      # read an item
eselect news read all                    # mark everything as read

# Caches and the database lock
sudo eselect cache clean --all -p        # dry run
sudo eselect cache clean --pkgcache --build -y
eselect db status
sudo eselect db unlock                   # refuses while pacman/emerge is running
```

## Modules

```
eselect <module> <action> [arguments]
eselect <module> help               # actions and options of the module
eselect <module> <action> --help    # man eselect-<module>-<action>
```

| Module | Actions | Notes |
|--------|---------|-------|
| **profile** | `list` `show` `set <N\|path>` | `/etc/portage/make.profile` → `/var/db/portage/profiles/…` |
| **repository** | `list [-i]` `enable` `disable` `remove [-f]` `add <name> <uri> [-p N] [--sig L]` `packages [-v] [-r] <name>` | `/etc/portage/repos.conf/eselect.conf` + key import; `packages` reads the sync database directly |
| **java** | `status` `get` `set <env>` `unset` `fix` | `/usr/lib/jvm/default`; also available as `archlinux-java <action>` |
| **news** | `list` `read [N\|all]` | Delegates to `emerge --news` |
| **world** | `show` `regen [-p] [-a]` | Adds explicitly installed packages that are missing from world |
| **cache** | `clean [--pkgcache] [--sources] [--packages] [--build] [--all] [-p] [-y]` | pacman cache + `~/.cache/aura-emerge/` of every user |
| **db** | `status` `lock` `unlock [-f]` | `/var/lib/pacman/db.lck` |

### profile

A profile is a leaf directory of the `/var/db/portage/profiles` tree that contains `packages`, `package.profile`, `package.mask`, `make.defaults` or `parent`.

`set` accepts a number from `list`, an absolute path or a path relative to the tree. The link is written as a relative one and replaced atomically.

If `make.profile` is a real directory or file, `set` asks and moves it to `make.profile.bak` — it never deletes it. Like the original, `set` prints nothing on success.

### java

A built-in replacement for `archlinux-java` (same layout as `java-environment-common`): an environment is a directory `/usr/lib/jvm/<env>` with an executable `bin/java`; if `<env>/jre/bin/java` exists, `<env>/jre` is listed as well.

| Action | What it does |
|--------|--------------|
| `status` | Lists environments, the current one is marked `(default)` |
| `get` | Prints the name of the current environment (an error if none is set) |
| `set <env>` | Makes an environment the current one: the `default` and `default-runtime` links (root) |
| `unset` | Removes both links (root) |
| `fix` | If `default` is unset or broken, picks the environment with the highest `java-N-…` number (JDK/JRE directories are preferred over `<env>/jre`); a working default is left alone, silent on success (root) |

When the `eselect` binary is reachable under the name `archlinux-java`, `archlinux-java <action>` is the same as `eselect java <action>`, so install scripts of Java packages keep working. You have to create the `/usr/bin/archlinux-java` link yourself: it conflicts with the `java-environment-common` package.

### repository packages

`eselect repository packages [-v] [-r] <name>` reads `/var/lib/pacman/sync/<name>.db` directly (zstd, gzip or plain tar; xz and bzip2 are not supported), without `pacman` or `emerge`, and prints one package per line — plain text, no colours or headings, so the output can be redirected to a file or `tee` (for example, to build profiles). `-v` appends the version, `-r` prefixes `<repo>/`. If the database is missing, the command tells you to synchronise and lists the available repositories.

### world

`show` prints the world file. `regen` reads explicitly installed packages directly from `/var/lib/pacman/local` (without calling `pacman`) and appends the ones missing from world. `-p` only shows them and needs no root, `-a` asks before writing.

### cache

| Flag | What is cleaned |
|------|-----------------|
| `--pkgcache` (`--pacman`) | `/var/cache/pacman/pkg` |
| `--sources` | `~/.cache/aura-emerge/sources` |
| `--packages` | `~/.cache/aura-emerge/packages` |
| `--build` | `~/.cache/aura-emerge/build` |
| `--all` | everything above |

`/root` and every `/home/*` are processed. Running as root, symlinks planted by a user are not followed: every path component below the home directory must be a real directory. `-p` is a dry run, `-y` skips the confirmation.

### db

`status` shows the lock, its age and running package managers; `lock` creates `db.lck`; `unlock` removes it and refuses while `pacman`, `emerge`, `paru`, `yay` or `pamac-daemon` is running (`-f` forces it).

## Repositories

AE reads `/etc/portage/repos.conf` (a file or `repos.conf/*.conf`, `FEATURES="repos"`). eselect owns exactly one file there:

```
/etc/portage/repos.conf/eselect.conf
```

* `enable` / `add` write a `[name]` block (keys AE understands: `priority`, `sync-uri`, `include`, `verify-signature`, `location`, `auto-sync`, `sync-type`).
* `disable` keeps the block as lines prefixed with `#| ` (comments for AE), so `enable` can restore it.
* `remove` deletes the block.
* A name that is already defined in another repos.conf file gives a warning and a question.
* If `repos.conf` is still a single file, eselect offers to turn it into a directory (the old file becomes `repos.conf/00-main.conf`).
* eselect warns when `FEATURES="repos"` is not set in `make.conf`.

`list` shows configured repositories (`*` — enabled; `(disabled)`; `(repos.conf)` — defined in another file) and, without `-i`, known ones that have keys or a template.

### Keys

On `eselect repository enable|add <name>` (first match wins):

1. `/usr/share/pacman/keyrings/<name>.gpg` (+ `-trusted` / `-revoked`) — the same steps as `pacman-key --populate <name>`
2. `/var/db/portage/repos/<name>/keys/<name>.gpg` (+ trusted/revoked)

The template for `enable` is optional: `/var/db/portage/repos/<name>/<name>.conf`.

```
/var/db/portage/repos/chaotic/
  chaotic.conf                 # the [chaotic] block that `enable` copies
  keys/
    chaotic.gpg
    chaotic-trusted            # FPR:LEVEL: lines (gpg ownertrust)
    chaotic-revoked            # optional, one FPR per line
```


## Files

| Path | Purpose |
|------|---------|
| `/etc/portage/make.profile` | Selected profile (symlink) |
| `/var/db/portage/profiles/` | Profile tree |
| `/etc/portage/repos.conf/eselect.conf` | Repositories managed by eselect |
| `/var/db/portage/repos/<name>/` | Repository template and keys |
| `/usr/share/pacman/keyrings/` | Keyrings (`<name>.gpg`) |
| `/var/lib/pacman/sync/<repo>.db` | Sync databases (for `repository packages`) |
| `/var/lib/pacman/local/` | Local database (for `world regen`) |
| `/usr/lib/jvm/` | Java environments and the `default`, `default-runtime` links |
| `/etc/portage/world` | The world file |
| `/etc/portage/make.conf` | Checked for `FEATURES="repos"` |
| `/var/lib/pacman/db.lck` | Database lock |

## Author / License

Copyright (C) 2026 **Undercat037**  
License GPL-3.0
