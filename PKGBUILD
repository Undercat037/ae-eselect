# Maintainer: Undercat037 <deltacatdeveloper@gmail.com>
pkgname=aeutils-eselect-git
pkgver=0.0.0
pkgrel=1
pkgdesc="A modular configuration tool in the style of Gentoo eselect for aura-emerge: select profile, repositories and their keys, the world file, caches, the package database lock and Java environments - with Portage-like output."
arch=('x86_64')
url="https://undercat037.github.io/aura-emerge/"
# Github repo: https://github.com/Undercat037/ae-eselect
# Gitlab repo: https://gitlab.com/Undercat037/ae-eselect
license=('GPL-3.0-only')
depends=('gnupg')
optdepends=(
  'aura-emerge: eselect core'
  'pacman: libalpm - package, cache, path and keyring layout'
)
makedepends=('rust' 'cargo')
conflicts=('portage' 'portage-git' 'eselect-git' 'aeutils-eselect')
provides=('eselect' 'aeutils-eselect')
install=aeutils-eselect.install
source=()
sha256sums=()

pkgver() {
  cd "${startdir:-${PWD}}" || return 1
  local ver rev short pkgid

  # path+file:///…/crate#3.0.28-beta
  # path+file:///…/crate#name@3.0.28-beta
  if command -v cargo >/dev/null 2>&1; then
    pkgid=$(cargo pkgid 2>/dev/null) || pkgid=
    if [[ -n $pkgid && $pkgid == *#* ]]; then
      if [[ $pkgid == *#*@* ]]; then
        ver=${pkgid##*@}
      else
        ver=${pkgid##*#}
      fi
      ver=${ver%%[[:space:]]*}
    fi
  fi

  if [[ -z $ver && -f Cargo.toml ]]; then
    ver=$(awk '
      /^\[package\]/ { in_pkg = 1; next }
      /^\[/ { in_pkg = 0 }
      in_pkg && $0 ~ /^[[:space:]]*version[[:space:]]*=/ {
        line = $0
        sub(/^[^=]*=[[:space:]]*/, "", line)
        gsub(/["\047]/, "", line)
        sub(/[[:space:]].*/, "", line)
        sub(/,.*/, "", line)
        print line
        exit
      }
    ' Cargo.toml)
  fi

  ver=${ver//-/_}
  [[ -n $ver ]] || return 1

  if rev=$(git rev-list --count HEAD 2>/dev/null) && short=$(git rev-parse --short HEAD 2>/dev/null); then
    printf '%s.r%s.g%s' "$ver" "$rev" "$short"
  else
    printf '%s' "$ver"
  fi
}

build() {
  cd "$startdir" || return 1
  cargo build --release --locked

  # man pages (eselect.1, eselect-<module>.1, eselect-<module>-<action>.1)
  # and completions come straight from the clap definition
  local bin="$PWD/target/release/eselect"
  rm -rf man comp
  mkdir -p man comp
  "$bin" --gen-manpage man
  "$bin" --gen-completions bash >comp/bash
  "$bin" --gen-completions zsh >comp/zsh
  "$bin" --gen-completions fish >comp/fish
  local f
  for f in man/eselect.1 comp/bash comp/zsh comp/fish; do
    [[ -s $f ]] || {
      error "empty $f - refusing to package it"
      return 1
    }
  done
}

package() {
  cd "$startdir" || return 1

  install -Dm755 target/release/eselect "$pkgdir/usr/bin/eselect"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
  install -Dm644 README.MD "$pkgdir/usr/share/doc/$pkgname/README.MD"
  install -Dm644 RU-README.MD "$pkgdir/usr/share/doc/$pkgname/RU-README.MD"
  install -Dm644 UA-README.MD "$pkgdir/usr/share/doc/$pkgname/UA-README.MD"

  install -Dm644 man/*.1 -t "$pkgdir/usr/share/man/man1/"

  install -Dm644 comp/bash "$pkgdir/usr/share/bash-completion/completions/eselect"
  install -Dm644 comp/zsh "$pkgdir/usr/share/zsh/site-functions/_eselect"
  install -Dm644 comp/fish "$pkgdir/usr/share/fish/vendor_completions.d/eselect.fish"

  # profile tree and repo templates/keys
  install -dm755 "$pkgdir/var/db/portage/profiles"
  install -dm755 "$pkgdir/var/db/portage/repos"
}
