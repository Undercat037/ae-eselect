<div align="center">
  <img src="misc/logo/ae-eselect_banner_386x140.svg" width="350" alt="ae-eselect logo">

# ae-eselect — an eselect module for aura-emerge

[![AE_Site](https://img.shields.io/badge/AE_Site-333333?style=flat-square&logo=bookstack&logoColor=FFFFFF)](https://undercat037.github.io/aura-emerge/)
[![GitHub](https://img.shields.io/badge/GitHub-333333?style=flat-square&logo=github&logoColor=FFFFFF)](https://github.com/Undercat037/ae-eselect)
[![GitLab](https://img.shields.io/badge/GitLab-333333?style=flat-square&logo=gitlab&logoColor=FC6D26)](https://gitlab.com/Undercat037/ae-eselect)

en | [ua](UA-README.MD) | [ru](RU-README.MD)
</div>

> A modular configuration tool in the style of Gentoo `eselect` for **aura-emerge** / **Cynosure**: the profile symlink, repositories and their keys, the world file, caches, the package database lock and Java environments — with Portage-like output.

As in Gentoo, `eselect` is a separate package, not a part of `emerge`. It is not a full port of the original but the multitool of the Cynosure ecosystem, so the set of modules and global options differs (for example, there is no `--root` or `--eprefix`).

**Sections:** [Installation](#installation) · [Usage examples](#usage-examples) · [Output and global options](#output-and-global-options) · [Modules](#modules) · [Repositories](#repositories) · [Files](#files)

## Installation

Through aura-emerge (recommended):
```bash
git clone https://github.com/Undercat037/ae-eselect
cd ae-eselect
emerge --usepkgbuild ./ --ask
```

Through makepkg:
```bash
git clone https://github.com/Undercat037/ae-eselect
cd ae-eselect
makepkg -si
```

Make:
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

> **Dependencies:** `gnupg` (key import for repositories from the database), `pacman` (libalpm), `curl` (for `reflector`), `aura-emerge` (eselect edits /etc/portage/, which is pointless without AE)

## Usage examples
```bash
# Overview
eselect                                  # global options and the list of modules
eselect profile                          # the actions of one module
eselect profile help                     # the same, with descriptions

# Profile
eselect profile list                     # numbered list, the current one is marked with *
eselect profile show                     # the current make.profile link
sudo eselect profile set 3               # by number from `list`
sudo eselect profile set default/linux/amd64/23.0   # or by path

# Repositories
eselect repository list                  # configured and known repositories
eselect repository list -i               # configured only
sudo eselect repository enable chaotic-aur   # from the local database + key import
sudo eselect repository add myrepo 'https://example.org/$repo/os/$arch' -p 60
sudo eselect repository disable myrepo   # the block stays, enable restores it
sudo eselect repository remove -f myrepo

# World
eselect world show
eselect world regen -p                   # what would be added
sudo eselect world regen -a              # add the missing ones, with confirmation

# Package list of a repository (plain text, can be redirected to a file)
eselect repository packages core > packages
eselect repository packages -r -v extra | tee extra.list   # extra/foo 1.2-1

# Mirrors: rank the mirrorlists of repositories (fastest first)
eselect reflector list                   # which mirrorlist belongs to which repository
eselect reflector -p core blackarch      # only show the ranking, write nothing
sudo eselect reflector core blackarch world system
sudo eselect reflector -n 5 --protocol all blackarch

# Java (built-in archlinux-java)
eselect java status                      # environments, the current one is marked (default)
sudo eselect java set java-21-openjdk
sudo eselect java fix                    # repair the default link

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

## Output and global options

The output follows Gentoo `eselect`: bold green list headings, a numbered `[N]` column, `*` marks the active entry, errors look like `!!! Error: …`.

| Option | Purpose |
|--------|---------|
| `--brief` | Shorter output: no headings (of lists and help) and no `*` marker |
| `--colour=yes\|no\|auto` | Enable or disable colours |


## Modules

```
eselect <module> <action> [arguments]
eselect <module> help               # the module's actions and options
eselect <module> <action> --help    # man eselect-<module>-<action>
```

| Module | Actions | Notes |
|--------|---------|-------|
| **profile** | `list` `show` `set <N\|path>` | `/etc/portage/make.profile` → `/var/db/portage/profiles/…` |
| **repository** | `list [-i]` `enable` `disable` `remove [-f]` `add <name> <uri> [-p N] [--sig L]` `packages [-v] [-r] <name>` | `/etc/portage/repos.conf/eselect.conf` + key and mirrorlist import; `packages` reads the sync database directly |
| **reflector** | `[-n N] [-t S] [-j N] [--protocol P] [--active-only] [-p] <repo>...` `list` | Ranks the mirrors of mirrorlists; no action word is needed, the repository names are enough |
| **java** | `status` `get` `set <env>` `unset` `fix` | `/usr/lib/jvm/default`; also available as `archlinux-java <action>` |
| **news** | `list` `read [N\|all]` | Delegates to `emerge --news` |
| **world** | `show` `regen [-p] [-a]` | Adds explicitly installed packages that are missing from world |
| **cache** | `clean [--pkgcache] [--sources] [--packages] [--build] [--all] [-p] [-y]` | The pacman cache + `~/.cache/aura-emerge/` of every user |
| **db** | `status` `lock` `unlock [-f]` | `/var/lib/pacman/db.lck` |

### profile

A profile is a leaf directory of the `/var/db/portage/profiles` tree that contains `packages`, `package.profile`, `package.mask`, `make.defaults` or `parent`.

`set` takes a number from `list`, an absolute path or a path relative to the tree. The link is written as a relative one and replaced atomically.

If `make.profile` is a real directory or file, `set` asks and moves it to `make.profile.bak` — it never deletes it. Like the original, `set` prints nothing on success.

### reflector

Ranks the mirrors of repositories and rewrites their mirrorlists: the fastest come first. No action word is needed: repository names right after the module already mean ranking — `eselect reflector core blackarch world system`.

Only the source that is in effect is read: with `FEATURES="repos"` in `make.conf` it is `/etc/portage/repos.conf` (`include =`), without it `/etc/pacman.conf` (`Include =`). Repositories that share one file (`core`, `extra`, `multilib` → `/etc/pacman.d/mirrorlist`) are ranked once. A repository with a single `sync-uri` or `Server` has nothing to rank — this is reported explicitly.

The candidates are all `Server =` lines of the file, **commented-out ones included**. Every mirror is probed with `curl`: the first 512 KiB of `<repo>.db` are downloaded and the time is recorded. For a shared file the first of the named repositories is used; `$repo`, `$arch`, `$arch_v3`, `$arch_v4` are expanded.

| Option | Purpose |
|--------|---------|
| `-n, --number <N>` | How many of the fastest mirrors stay active (10) |
| `-t, --timeout <S>` | Total probe time of one mirror (connect and download), seconds (5) |
| `-j, --jobs <N>` | Parallel probes (8) |
| `--protocol <P>` | `https` (default), `http` or `all` |
| `--active-only` | Only consider the mirrors that are active now |
| `-p, --pretend` | Show the ranking, write nothing (no root needed) |

If a server ignores ranges (`Range`), it sends the whole file, and a large `.db` may not fit into the timeout — such a mirror ends up among the unreachable ones. `reflector list` shows the file and the number of active/total mirrors for every repository.

### java

A built-in replacement for `archlinux-java` (the layout of `java-environment-common`): an environment is a `/usr/lib/jvm/<env>` directory with an executable `bin/java`; if `<env>/jre/bin/java` exists, `<env>/jre` is listed as well.

| Action | What it does |
|--------|--------------|
| `status` | Lists the environments, the current one is marked `(default)` |
| `get` | Prints the name of the current environment (an error if none is set) |
| `set <env>` | Makes the environment current: the `default` and `default-runtime` links (root) |
| `unset` | Removes both links (root) |
| `fix` | If `default` is unset or broken, picks the environment with the highest `java-N-…` number (JDK/JRE directories are preferred over `<env>/jre`); leaves a working one alone and prints nothing on success (root) |

If the `eselect` binary is reachable as `archlinux-java`, then `archlinux-java <action>` equals `eselect java <action>`, so the install scripts of Java packages keep working. You have to create the `/usr/bin/archlinux-java` link yourself: it conflicts with the `java-environment-common` package.

### repository packages

`eselect repository packages [-v] [-r] <name>` reads `/var/lib/pacman/sync/<name>.db` directly (zstd, gzip or plain tar; xz and bzip2 are not supported), without `pacman` and `emerge`, and prints one package per line — plain text, no colours or headings, so the output can be redirected to a file or `tee` (for example, to create profiles). `-v` appends the version, `-r` adds the `<repo>/` prefix. If the database is missing, the command suggests running a sync and lists the available repositories.

### world

`show` prints the world file. `regen` reads explicitly installed packages straight from `/var/lib/pacman/local` (without calling `pacman`) and appends the ones missing from world. `-p` only shows them and needs no root, `-a` asks before writing.

### cache

| Flag | What is cleaned |
|------|-----------------|
| `--pkgcache` (`--pacman`) | `/var/cache/pacman/pkg` |
| `--sources` | `~/.cache/aura-emerge/sources` |
| `--packages` | `~/.cache/aura-emerge/packages` |
| `--build` | `~/.cache/aura-emerge/build` |
| `--all` | everything listed above |

`/root` and every `/home/*` are processed. Under root, symlinks planted by a user are not followed: every path component below the home directory must be a real directory. `-p` is a dry run, `-y` skips the confirmation.

### db

`status` shows the lock, its age and the running package managers; `lock` creates `db.lck`; `unlock` removes it and refuses while `pacman`, `emerge`, `paru`, `yay` or `pamac-daemon` is running (`-f` forces it).

## Repositories

AE reads `/etc/portage/repos.conf` (a file or `repos.conf/*.conf`, `FEATURES="repos"`). eselect owns exactly one file there:

```
/etc/portage/repos.conf/eselect.conf
```

* `enable` / `add` write a `[name]` block (the keys AE understands: `priority`, `sync-uri`, `include`, `verify-signature`, `location`, `auto-sync`, `sync-type`).
* `disable` keeps the block as lines prefixed with `#| ` (comments for AE), so `enable` can restore it.
* `remove` deletes the block.
* A name that is already defined in another repos.conf file produces a warning and a question.
* If `repos.conf` is still a single file, eselect offers to turn it into a directory (the old file becomes `repos.conf/00-main.conf`).
* eselect warns if `FEATURES="repos"` is not set in `make.conf`.

`list` shows the configured repositories (`*` — enabled; `(disabled)`; `(repos.conf)` — defined in another file) and, without `-i`, the known ones that have keys or a template.

### Keys

On `eselect repository enable|add <name>` (the first match wins):

1. `/usr/share/pacman/keyrings/<name>.gpg` (+ `-trusted` / `-revoked`) — the same steps as `pacman-key --populate <name>`
2. `/var/db/portage/repos/<name>/keys/<name>.gpg` (+ trusted/revoked)

The template for `enable` is optional: `/var/db/portage/repos/<name>/<name>.conf`. A `mirrorlist` can be put next to it: if the file from the template's `include =` does not exist yet, `enable` installs it from there; if there is no template, a warning with a hint is printed (`eselect reflector <name>`).

```
/var/db/portage/repos/chaotic/
  chaotic.conf                 # the [chaotic] block that `enable` copies
  mirrorlist                   # optional: installed to the include path if it is missing
  keys/
    chaotic.gpg
    chaotic-trusted            # FPR:LEVEL: lines (gpg ownertrust)
    chaotic-revoked            # optional, one FPR per line
```


## Files

| Path | Purpose |
|------|---------|
| `/etc/portage/make.profile` | The selected profile (symlink) |
| `/var/db/portage/profiles/` | The profile tree |
| `/etc/portage/repos.conf/eselect.conf` | Repositories managed by eselect |
| `/var/db/portage/repos/<name>/` | Repository template and keys |
| `/usr/share/pacman/keyrings/` | Keyrings (`<name>.gpg`) |
| `/etc/pacman.d/*mirrorlist*` | Mirrorlists of repositories (rewritten by `reflector`, a copy is kept as `*.eselect-bak`) |
| `/var/lib/pacman/sync/<repo>.db` | Sync databases (for `repository packages`) |
| `/var/lib/pacman/local/` | The local database (for `world regen`) |
| `/usr/lib/jvm/` | Java environments and the `default`, `default-runtime` links |
| `/etc/portage/world` | The world file |
| `/etc/portage/make.conf` | Checked for `FEATURES="repos"` |
| `/var/lib/pacman/db.lck` | The database lock |

## Author / License

Copyright (C) 2026 **Undercat037**  
License GPL-3.0
