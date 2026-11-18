<div align="center">
  <img src="misc/logo/ae-eselect_banner_386x140.svg" width="350" alt="ae-eselect logo">

# ae-eselect — eselect-модуль для aura-emerge

[![AE_Site](https://img.shields.io/badge/AE_Site-333333?style=flat-square&logo=bookstack&logoColor=FFFFFF)](https://undercat037.github.io/aura-emerge/)
[![GitHub](https://img.shields.io/badge/GitHub-333333?style=flat-square&logo=github&logoColor=FFFFFF)](https://github.com/Undercat037/ae-eselect)
[![GitLab](https://img.shields.io/badge/GitLab-333333?style=flat-square&logo=gitlab&logoColor=FC6D26)](https://gitlab.com/Undercat037/ae-eselect)

[en](README.MD) | ua | [ru](RU-README.MD)
</div>

> Модульний інструмент налаштування у стилі Gentoo `eselect` для **aura-emerge** / **Cynosure**: символічне посилання профілю, репозиторії та їхні ключі, файл world, кеші, блокування бази пакунків і середовища Java — з виводом як у Portage.

Як і в Gentoo, `eselect` — окремий пакунок, а не частина `emerge`. Це не повний порт оригіналу, а власний мультитул екосистеми Cynosure, тому набір модулів і глобальних опцій інший (наприклад, немає `--root` та `--eprefix`).

**Розділи:** [Встановлення](#встановлення) · [Приклади використання](#приклади-використання) · [Вивід і глобальні опції](#вивід-і-глобальні-опції) · [Модулі](#модулі) · [Репозиторії](#репозиторії) · [Файли](#файли)

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

> **Залежності:** `gnupg` (ключі для імпорту репозиторіїв з бази), `pacman` (libalpm), `curl` (для `reflector`), `aura-emerge` (eselect править /etc/portage/, без AE це безглуздо)

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

# World
eselect world show
eselect world regen -p                   # що буде додано
sudo eselect world regen -a              # додати відсутні, з підтвердженням

# Список пакунків репозиторію (чистий текст, можна писати у файл)
eselect repository packages core > packages
eselect repository packages -r -v extra | tee extra.list   # extra/foo 1.2-1

# Дзеркала: ранжувати mirrorlist-и репозиторіїв (найшвидші — першими)
eselect reflector list                   # який mirrorlist у якого репозиторію
eselect reflector -p core blackarch      # лише показати рейтинг, нічого не писати
sudo eselect reflector core blackarch world system
sudo eselect reflector -n 5 --protocol all blackarch

# Java (вбудований archlinux-java)
eselect java status                      # середовища, поточне позначено (default)
sudo eselect java set java-21-openjdk
sudo eselect java fix                    # полагодити посилання default

# Новини
eselect news                             # Arch News
eselect news read 8                      # прочитати новину
eselect news read all                    # позначити всі прочитаними

# Кеші та блокування бази
sudo eselect cache clean --all -p        # пробний запуск
sudo eselect cache clean --pkgcache --build -y
eselect db status
sudo eselect db unlock                   # відмовить, поки працює pacman/emerge
```

## Вивід і глобальні опції

Вивід повторює Gentoo `eselect`: зелені жирні заголовки списків, нумерований стовпець `[N]`, `*` позначає активний запис, помилки виглядають як `!!! Error: …`.

| Опція | Призначення |
|-------|-------------|
| `--brief` | Коротший вивід: без заголовків (списків і довідки) та без маркера `*` |
| `--colour=yes\|no\|auto` | Увімкнути/вимкнути кольори |


## Модулі

```
eselect <модуль> <дія> [аргументи]
eselect <модуль> help               # дії та опції модуля
eselect <модуль> <дія> --help       # man eselect-<модуль>-<дія>
```

| Модуль | Дії | Примітки |
|--------|-----|----------|
| **profile** | `list` `show` `set <N\|path>` | `/etc/portage/make.profile` → `/var/db/portage/profiles/…` |
| **repository** | `list [-i]` `enable` `disable` `remove [-f]` `add <name> <uri> [-p N] [--sig L]` `packages [-v] [-r] <name>` | `/etc/portage/repos.conf/eselect.conf` + імпорт ключів і mirrorlist; `packages` читає sync-базу напряму |
| **reflector** | `[-n N] [-t S] [-j N] [--protocol P] [--active-only] [-p] <repo>...` `list` | Ранжує дзеркала mirrorlist-ів; окрема дія не потрібна — досить назв репозиторіїв |
| **java** | `status` `get` `set <env>` `unset` `fix` | `/usr/lib/jvm/default`; доступно й як `archlinux-java <дія>` |
| **news** | `list` `read [N\|all]` | Делегує до `emerge --news` |
| **world** | `show` `regen [-p] [-a]` | Додає пакунки з explicit, яких немає у world |
| **cache** | `clean [--pkgcache] [--sources] [--packages] [--build] [--all] [-p] [-y]` | Кеш pacman + `~/.cache/aura-emerge/` усіх користувачів |
| **db** | `status` `lock` `unlock [-f]` | `/var/lib/pacman/db.lck` |

### profile

Профіль — це кінцевий каталог дерева `/var/db/portage/profiles`, у якому є `packages`, `package.profile`, `package.mask`, `make.defaults` або `parent`.

`set` приймає номер із `list`, абсолютний шлях або шлях відносно дерева. Посилання записується відносним і підмінюється атомарно.

Якщо `make.profile` — справжній каталог або файл, `set` запитає й перенесе його до `make.profile.bak` — ніколи не видаляє. Як і оригінал, `set` у разі успіху нічого не виводить.

### reflector

Ранжує дзеркала репозиторіїв і переписує їхні mirrorlist-и: найшвидші йдуть першими. Дію вказувати не потрібно: назви репозиторіїв одразу після модуля вже означають ранжування — `eselect reflector core blackarch world system`.

Читається лише те джерело, що діє: при `FEATURES="repos"` у `make.conf` — `/etc/portage/repos.conf` (`include =`), без нього — `/etc/pacman.conf` (`Include =`). Репозиторії, що ділять один файл (`core`, `extra`, `multilib` → `/etc/pacman.d/mirrorlist`), ранжуються один раз. У репозиторію з єдиним `sync-uri` або `Server` ранжувати нічого — про це повідомляється явно.

Кандидати — усі рядки `Server =` файлу, **включно із закоментованими**. Кожне дзеркало перевіряється через `curl`: завантажуються перші 512 КіБ файлу `<repo>.db`, і враховується час. Для спільного файлу береться перший із зазначених репозиторіїв; `$repo`, `$arch`, `$arch_v3`, `$arch_v4` підставляються.

| Опція | Призначення |
|-------|-------------|
| `-n, --number <N>` | Скільки найшвидших дзеркал залишити активними (10) |
| `-t, --timeout <S>` | Повний час перевірки одного дзеркала (з'єднання та завантаження), секунди (5) |
| `-j, --jobs <N>` | Паралельних перевірок (8) |
| `--protocol <P>` | `https` (типово), `http` або `all` |
| `--active-only` | Враховувати лише дзеркала, активні зараз |
| `-p, --pretend` | Показати рейтинг, нічого не писати (root не потрібен) |

Якщо сервер ігнорує діапазони (`Range`), він віддає файл повністю, і великий `.db` може не вкластися в таймаут — таке дзеркало потрапить до недоступних. `reflector list` показує файл і кількість активних/усіх дзеркал для кожного репозиторію.

### java

Вбудована заміна `archlinux-java` (розкладка як у `java-environment-common`): середовище — каталог `/usr/lib/jvm/<env>` з виконуваним `bin/java`; якщо є `<env>/jre/bin/java`, у списку з'являється й `<env>/jre`.

| Дія | Що робить |
|-----|-----------|
| `status` | Список середовищ, поточне позначено `(default)` |
| `get` | Виводить назву поточного середовища (помилка, якщо не задано) |
| `set <env>` | Робить середовище поточним: посилання `default` і `default-runtime` (root) |
| `unset` | Видаляє обидва посилання (root) |
| `fix` | Якщо `default` не задано або зламано, обирає середовище з найбільшим номером `java-N-…` (каталоги JDK/JRE переважають над `<env>/jre`); справне не чіпає, у разі успіху мовчить (root) |

Якщо бінарник `eselect` доступний під назвою `archlinux-java`, то `archlinux-java <дія>` рівнозначно `eselect java <дія>`, тож скрипти встановлення Java-пакунків продовжують працювати. Посилання `/usr/bin/archlinux-java` потрібно створити самостійно: воно конфліктує з пакунком `java-environment-common`.

### repository packages

`eselect repository packages [-v] [-r] <name>` читає `/var/lib/pacman/sync/<name>.db` напряму (zstd, gzip або звичайний tar; xz і bzip2 не підтримуються), без `pacman` та `emerge`, і друкує по одному пакунку в рядку — лише текст, без кольорів і заголовків, щоб вивід можна було спрямувати у файл або `tee` (наприклад, для створення профілів). `-v` додає версію, `-r` — префікс `<repo>/`. Якщо бази немає, команда підказує виконати синхронізацію та перелічує доступні репозиторії.

### world

`show` виводить файл world. `regen` читає явно встановлені пакунки напряму з `/var/lib/pacman/local` (без виклику `pacman`) і дописує ті, яких у world немає. `-p` лише показує їх і не потребує root, `-a` питає перед записом.

### cache

| Прапор | Що чиститься |
|--------|--------------|
| `--pkgcache` (`--pacman`) | `/var/cache/pacman/pkg` |
| `--sources` | `~/.cache/aura-emerge/sources` |
| `--packages` | `~/.cache/aura-emerge/packages` |
| `--build` | `~/.cache/aura-emerge/build` |
| `--all` | усе перелічене |

Обробляються `/root` і кожен `/home/*`. Під root символічні посилання, підкинуті користувачем, не розкриваються: кожен компонент шляху нижче домашнього каталогу має бути справжнім каталогом. `-p` — пробний запуск, `-y` — без підтвердження.

### db

`status` показує блокування, його вік і запущені менеджери пакунків; `lock` створює `db.lck`; `unlock` видаляє його й відмовляє, поки працює `pacman`, `emerge`, `paru`, `yay` або `pamac-daemon` (`-f` примусово).

## Репозиторії

AE читає `/etc/portage/repos.conf` (файл або `repos.conf/*.conf`, `FEATURES="repos"`). eselect володіє там рівно одним файлом:

```
/etc/portage/repos.conf/eselect.conf
```

* `enable` / `add` пишуть блок `[name]` (ключі, які розуміє AE: `priority`, `sync-uri`, `include`, `verify-signature`, `location`, `auto-sync`, `sync-type`).
* `disable` залишає блок рядками з префіксом `#| ` (для AE це коментарі), тому `enable` може його відновити.
* `remove` видаляє блок.
* Назва, що вже визначена в іншому файлі repos.conf, дає попередження й питання.
* Якщо `repos.conf` досі один файл, eselect пропонує перетворити його на каталог (старий файл стає `repos.conf/00-main.conf`).
* eselect попереджає, якщо в `make.conf` не задано `FEATURES="repos"`.

`list` показує налаштовані репозиторії (`*` — увімкнено; `(disabled)`; `(repos.conf)` — визначено в іншому файлі) і, без `-i`, відомі, для яких є ключі або шаблон.

### Ключі

При `eselect repository enable|add <name>` (перший збіг виграє):

1. `/usr/share/pacman/keyrings/<name>.gpg` (+ `-trusted` / `-revoked`) — ті самі кроки, що й `pacman-key --populate <name>`
2. `/var/db/portage/repos/<name>/keys/<name>.gpg` (+ trusted/revoked)

Шаблон для `enable` необов'язковий: `/var/db/portage/repos/<name>/<name>.conf`. Поруч можна покласти `mirrorlist`: якщо файлу з `include =` шаблону ще не існує, `enable` встановить його звідти; якщо шаблону немає, виводиться попередження з підказкою (`eselect reflector <name>`).

```
/var/db/portage/repos/chaotic/
  chaotic.conf                 # блок [chaotic], який копіює `enable`
  mirrorlist                   # необов'язково: ставиться в include-шлях, якщо його немає
  keys/
    chaotic.gpg
    chaotic-trusted            # рядки FPR:LEVEL: (gpg ownertrust)
    chaotic-revoked            # необов'язково, по одному FPR в рядку
```


## Файли

| Шлях | Призначення |
|------|-------------|
| `/etc/portage/make.profile` | Обраний профіль (символічне посилання) |
| `/var/db/portage/profiles/` | Дерево профілів |
| `/etc/portage/repos.conf/eselect.conf` | Репозиторії, якими керує eselect |
| `/var/db/portage/repos/<name>/` | Шаблон і ключі репозиторію |
| `/usr/share/pacman/keyrings/` | Keyring-и (`<name>.gpg`) |
| `/etc/pacman.d/*mirrorlist*` | Mirrorlist-и репозиторіїв (їх переписує `reflector`, копія `*.eselect-bak`) |
| `/var/lib/pacman/sync/<repo>.db` | Sync-бази (для `repository packages`) |
| `/var/lib/pacman/local/` | Локальна база (для `world regen`) |
| `/usr/lib/jvm/` | Середовища Java та посилання `default`, `default-runtime` |
| `/etc/portage/world` | Файл world |
| `/etc/portage/make.conf` | Перевіряється на `FEATURES="repos"` |
| `/var/lib/pacman/db.lck` | Блокування бази |

## Автор / Ліцензія

Copyright (C) 2026 **Undercat037**  
Ліцензія GPL-3.0
