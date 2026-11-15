<div align="center">
  <img src="misc/logo/ae-eselect_banner_386x140.svg" width="350" alt="ae-eselect logo">

# ae-eselect — eselect-модуль для aura-emerge

[![AE_Site](https://img.shields.io/badge/AE_Site-333333?style=flat-square&logo=bookstack&logoColor=FFFFFF)](https://undercat037.github.io/aura-emerge/)
[![GitHub](https://img.shields.io/badge/GitHub-333333?style=flat-square&logo=github&logoColor=FFFFFF)](https://github.com/Undercat037/ae-eselect)
[![GitLab](https://img.shields.io/badge/GitLab-333333?style=flat-square&logo=gitlab&logoColor=FC6D26)](https://gitlab.com/Undercat037/ae-eselect)

[en](README.MD) | ua | [ru](RU-README.MD)
</div>

> Модульний інструмент налаштування в стилі Gentoo `eselect` для **aura-emerge**: вибір профілю, репозиторії та їхні ключі, файл world, кеші, блокування бази пакунків і середовища Java - з Portage-подібним виводом.

Як і в Gentoo, `eselect` — окремий пакунок, а не частина `emerge`. Це не повний порт оригіналу, а власний мультитул в доповнення до aura-emerge.

**Розділи:** [Встановлення](#встановлення) · [Приклади використання](#приклади-використання) · [Модулі](#модулі) · [Репозиторії](#репозиторії) · [Файли](#файли)

## Встановлення

Через aura-emerge (рекомендовано):
```bash
git clone https://github.com/Undercat037/ae-eselect
cd ae-eselect
emerge --usepkgbuild ./ --ask
```

Через makepkg:
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
<summary><h3>Вручну</h3></summary>

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

> **Залежності:** `gnupg` `pacman` (libalpm)  
> **Опційно:** `aura-emerge`

## Приклади використання

```bash
# Огляд
eselect                                  # глобальні опції та список модулів
eselect profile                          # дії одного модуля
eselect profile help                     # те саме, з описом

# Профіль
eselect profile list                     # нумерований список, поточний позначено *
eselect profile show                     # поточне посилання make.profile
sudo eselect profile set 3               # за номером із `list`
sudo eselect profile set default/linux/amd64/23.0   # або за шляхом

# Репозиторії
eselect repository list                  # налаштовані та відомі репозиторії
eselect repository list -i               # лише налаштовані
sudo eselect repository enable chaotic-aur   # з локальної бази + імпорт ключів
sudo eselect repository add myrepo 'https://example.org/$repo/os/$arch' -p 60
sudo eselect repository disable myrepo   # блок лишається, enable його відновить
sudo eselect repository remove -f myrepo

# Список пакунків репозиторію (чистий текст, можна писати у файл)
eselect repository packages core > packages
eselect repository packages -r -v extra | tee extra.list   # extra/foo 1.2-1

# Java (вбудований archlinux-java)
eselect java status                      # середовища, поточне позначено (default)
sudo eselect java set java-21-openjdk
sudo eselect java fix                    # полагодити посилання default

# World
eselect world show
eselect world regen -p                   # що буде додано
sudo eselect world regen -a              # додати відсутні, із підтвердженням

# Новини
eselect news                             # Arch News
eselect news read 8                      # прочитати новину
eselect news read all                    # позначити прочитаними всі

# Кеші та блокування бази
sudo eselect cache clean --all -p        # пробний запуск
sudo eselect cache clean --pkgcache --build -y
eselect db status
sudo eselect db unlock                   # відмовить, поки працює pacman/emerge
```

## Модулі

```
eselect <модуль> <дія> [аргументи]
eselect <модуль> help               # дії та опції модуля
eselect <модуль> <дія> --help       # man eselect-<модуль>-<дія>
```

| Модуль | Дії | Примітки |
|--------|-----|----------|
| **profile** | `list` `show` `set <N\|path>` | `/etc/portage/make.profile` → `/var/db/portage/profiles/…` |
| **repository** | `list [-i]` `enable` `disable` `remove [-f]` `add <name> <uri> [-p N] [--sig L]` `packages [-v] [-r] <name>` | `/etc/portage/repos.conf/eselect.conf` + імпорт ключів; `packages` читає sync-базу напряму |
| **java** | `status` `get` `set <env>` `unset` `fix` | `/usr/lib/jvm/default`; доступно і як `archlinux-java <дія>` |
| **news** | `list` `read [N\|all]` | Делегує в `emerge --news` |
| **world** | `show` `regen [-p] [-a]` | Додає явно встановлені пакунки, яких немає у world |
| **cache** | `clean [--pkgcache] [--sources] [--packages] [--build] [--all] [-p] [-y]` | Кеш pacman + `~/.cache/aura-emerge/` усіх користувачів |
| **db** | `status` `lock` `unlock [-f]` | `/var/lib/pacman/db.lck` |

### profile

Профіль — це кінцевий каталог дерева `/var/db/portage/profiles`, у якому є `packages`, `package.profile`, `package.mask`, `make.defaults` або `parent`.

`set` приймає номер із `list`, абсолютний шлях або шлях відносно дерева. Посилання записується відносним і підміняється атомарно.

Якщо `make.profile` — справжній каталог або файл, `set` запитає й перенесе його до `make.profile.bak` — ніколи не видаляє. Як і оригінал, `set` у разі успіху нічого не виводить.

### java

Вбудована заміна `archlinux-java` (розкладка як у `java-environment-common`): середовище — це каталог `/usr/lib/jvm/<env>` з виконуваним `bin/java`; якщо є `<env>/jre/bin/java`, у списку з'являється й `<env>/jre`.

| Дія | Що робить |
|-----|-----------|
| `status` | Список середовищ, поточне позначено `(default)` |
| `get` | Виводить назву поточного середовища (помилка, якщо не задано) |
| `set <env>` | Робить середовище поточним: посилання `default` і `default-runtime` (root) |
| `unset` | Видаляє обидва посилання (root) |
| `fix` | Якщо `default` не задано або зламано, обирає середовище з найбільшим номером `java-N-…` (каталоги JDK/JRE кращі за `<env>/jre`); справне не чіпає, за успіху мовчить (root) |

Якщо бінарник `eselect` доступний під назвою `archlinux-java`, то `archlinux-java <дія>` рівнозначно `eselect java <дія>`, тому скрипти встановлення Java-пакунків продовжують працювати. Посилання `/usr/bin/archlinux-java` потрібно створити самостійно: воно конфліктує з пакунком `java-environment-common`.

### repository packages

`eselect repository packages [-v] [-r] <name>` читає `/var/lib/pacman/sync/<name>.db` напряму (zstd, gzip або звичайний tar; xz і bzip2 не підтримуються), без `pacman` та `emerge`, і друкує по одному пакунку в рядку — лише текст, без кольорів і заголовків, щоб вивід можна було спрямувати у файл або `tee` (наприклад, для створення профілів). `-v` додає версію, `-r` — префікс `<repo>/`. Якщо бази немає, команда підказує виконати синхронізацію та перелічує доступні репозиторії.

### world

`show` виводить файл world. `regen` читає явно встановлені пакунки напряму з `/var/lib/pacman/local` (без виклику `pacman`) і дописує ті, яких у world немає. `-p` лише показує їх і не потребує root, `-a` питає перед записом.

### cache

| Прапорець | Що чиститься |
|-----------|--------------|
| `--pkgcache` (`--pacman`) | `/var/cache/pacman/pkg` |
| `--sources` | `~/.cache/aura-emerge/sources` |
| `--packages` | `~/.cache/aura-emerge/packages` |
| `--build` | `~/.cache/aura-emerge/build` |
| `--all` | усе перелічене |

Обробляються `/root` і кожен `/home/*`. Під root символічні посилання, підкинуті користувачем, не розкриваються: кожен компонент шляху нижче домашнього каталогу має бути справжнім каталогом. `-p` — пробний запуск, `-y` — без підтвердження.

### db

`status` показує блокування, його вік і запущені менеджери пакунків; `lock` створює `db.lck`; `unlock` видаляє його та відмовляє, поки працює `pacman`, `emerge`, `paru`, `yay` або `pamac-daemon` (`-f` примусово).

## Репозиторії

AE читає `/etc/portage/repos.conf` (файл або `repos.conf/*.conf`, `FEATURES="repos"`). eselect володіє там рівно одним файлом:

```
/etc/portage/repos.conf/eselect.conf
```

* `enable` / `add` пишуть блок `[name]` (ключі, які розуміє AE: `priority`, `sync-uri`, `include`, `verify-signature`, `location`, `auto-sync`, `sync-type`).
* `disable` лишає блок рядками з префіксом `#| ` (для AE це коментарі), тому `enable` може його відновити.
* `remove` видаляє блок.
* Ім'я, що вже визначене в іншому файлі repos.conf, дає попередження та запитання.
* Якщо `repos.conf` усе ще один файл, eselect пропонує перетворити його на каталог (старий файл стає `repos.conf/00-main.conf`).
* eselect попереджає, якщо в `make.conf` не задано `FEATURES="repos"`.

`list` показує налаштовані репозиторії (`*` — увімкнено; `(disabled)`; `(repos.conf)` — визначено в іншому файлі) та, без `-i`, відомі, для яких є ключі або шаблон.

### Ключі

Під час `eselect repository enable|add <name>` (перший збіг виграє):

1. `/usr/share/pacman/keyrings/<name>.gpg` (+ `-trusted` / `-revoked`) — ті самі кроки, що й `pacman-key --populate <name>`
2. `/var/db/portage/repos/<name>/keys/<name>.gpg` (+ trusted/revoked)

Шаблон для `enable` необов'язковий: `/var/db/portage/repos/<name>/<name>.conf`.

```
/var/db/portage/repos/chaotic/
  chaotic.conf                 # блок [chaotic], який копіює `enable`
  keys/
    chaotic.gpg
    chaotic-trusted            # рядки FPR:LEVEL: (gpg ownertrust)
    chaotic-revoked            # необов'язково, по одному FPR у рядку
```


## Файли

| Шлях | Призначення |
|------|-------------|
| `/etc/portage/make.profile` | Обраний профіль (символічне посилання) |
| `/var/db/portage/profiles/` | Дерево профілів |
| `/etc/portage/repos.conf/eselect.conf` | Репозиторії, якими керує eselect |
| `/var/db/portage/repos/<name>/` | Шаблон і ключі репозиторію |
| `/usr/share/pacman/keyrings/` | Keyring-и (`<name>.gpg`) |
| `/var/lib/pacman/sync/<repo>.db` | Sync-бази (для `repository packages`) |
| `/var/lib/pacman/local/` | Локальна база (для `world regen`) |
| `/usr/lib/jvm/` | Середовища Java та посилання `default`, `default-runtime` |
| `/etc/portage/world` | Файл world |
| `/etc/portage/make.conf` | Перевіряється на `FEATURES="repos"` |
| `/var/lib/pacman/db.lck` | Блокування бази |

## Автор / Ліцензія

Copyright (C) 2026 **Undercat037**  
Ліцензія GPL-3.0
