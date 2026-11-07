# ae-eselect

Modular configuration tool for **aura-emerge** / **Cynosure**.  
Separate package from `emerge` (Gentoo-style: eselect is not part of Portage).

```
eselect <module> <action> [args]
eselect <module> --help        # man eselect-<module>-<action>
```

## Modules

| Module | Actions | Notes |
|--------|---------|--------|
| **profile** | `list` `show` `set <N\|path>` | `/etc/portage/make.profile` → `/var/db/portage/profiles/…` (a real dir there is moved to `make.profile.bak`, never deleted) |
| **repository** | `list [-i]` `enable` `disable` `remove [-f]` `add <name> <uri> [-p N] [--sig L]` | `/etc/portage/repos.conf/eselect.conf` + key import |
| **news** | `list` `read [N\|all]` | Delegates to `emerge --news` |
| **world** | `show` `regen [-p] [-a]` | Adds `pacman -Qeq` packages missing from world; `-p` needs no root |
| **cache** | `clean [--pacman] [--sources] [--packages] [--build] [--all] [-p] [-y]` | Pacman cache + `~/.cache/aura-emerge/{sources,packages,build}` of every user |
| **db** | `status` `lock` `unlock [-f]` | `/var/lib/pacman/db.lck`; `unlock` refuses while pacman/emerge runs |

## Repositories

AE reads `/etc/portage/repos.conf` (file or `repos.conf/*.conf`, `FEATURES="repos"`).
eselect owns exactly one file there:

```
/etc/portage/repos.conf/eselect.conf
```

* `enable` / `add` write a `[name]` block (keys AE understands: `priority`,
  `sync-uri`, `include`, `verify-signature`, `location`, `auto-sync`, `sync-type`).
* `disable` keeps the block as `#| `-prefixed lines (comments for AE) so `enable` can restore it.
* `remove` deletes the block.
* A name that is already defined in another repos.conf file triggers a warning and a question.
* If `repos.conf` is still a single file, eselect offers to turn it into a directory
  (old file → `repos.conf/00-main.conf`).
* eselect warns when `FEATURES="repos"` is not set in `make.conf`.

### Keys

On `eselect repository enable|add <name>` (first hit wins):

1. `/usr/share/pacman/keyrings/<name>.gpg` (+ `-trusted` / `-revoked`) — same steps as `pacman-key --populate <name>`
2. `/var/db/portage/repos/<name>/keys/<name>.gpg` (+ trusted/revoked)

Template for `enable` (optional): `/var/db/portage/repos/<name>/<name>.conf`

```
/var/db/portage/repos/chaotic/
  chaotic.conf                 # [chaotic] block copied by `enable`
  keys/
    chaotic.gpg
    chaotic-trusted            # FPR:LEVEL: lines (gpg ownertrust)
    chaotic-revoked            # optional, one FPR per line
```

## Man pages and completions

Generated from the clap definition (nothing hand-written):

```bash
eselect --gen-manpage DIR          # eselect.1, eselect-<module>.1, eselect-<module>-<action>.1
eselect --gen-manpage              # main page to stdout
eselect --gen-completions bash|zsh|fish|elvish|powershell
```

Completion is git-style: module → action → options.

## Build / install

```bash
make install          # build + man + completions + /usr/bin/eselect
# or as a package (PKGBUILD in this dir):
makepkg -si
```

Runtime: `gpg`, `pacman`; optional `emerge` (news).

## Relation to AE

`aura-emerge` ships a minimal `eselect profile` shim. Both coexist without
file conflicts:

* The shim is a symlink `/usr/local/bin/eselect -> /usr/bin/emerge` (plus
  completions under `/usr/local/share`), created by the AE package scriptlet
  **only if `/usr/bin/eselect` does not exist**.
* Installing `ae-eselect` puts the real binary in `/usr/bin/eselect` and
  removes the shim; removing it brings the shim back.
* Installing AE while ae-eselect is present never overwrites it.
* The helper is `/usr/lib/aura-emerge/eselect-shim install|remove`.
