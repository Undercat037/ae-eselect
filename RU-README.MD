<div align="center">
  <img src="misc/logo/ae-eselect_banner_386x140.svg" width="350" alt="ae-eselect logo">

# ae-eselect — eselect модуль для aura-emerge

[![AE_Site](https://img.shields.io/badge/AE_Site-333333?style=flat-square&logo=bookstack&logoColor=FFFFFF)](https://undercat037.github.io/aura-emerge/)
[![GitHub](https://img.shields.io/badge/GitHub-333333?style=flat-square&logo=github&logoColor=FFFFFF)](https://github.com/Undercat037/ae-eselect)
[![GitLab](https://img.shields.io/badge/GitLab-333333?style=flat-square&logo=gitlab&logoColor=FC6D26)](https://gitlab.com/Undercat037/ae-eselect)

[en](README.MD) | [ua](UA-README.MD) | ru
</div>

> Модульный инструмент настройки в стиле Gentoo `eselect` для **aura-emerge** / **Cynosure**: символическая ссылка профиля, репозитории и их ключи, файл world, кеши, блокировка базы пакетов и окружения Java — с выводом как в Portage.

Как и в Gentoo, `eselect` — отдельный пакет, а не часть `emerge`. Это не полный порт оригинала, а собственный мультитул экосистемы Cynosure, поэтому набор модулей и глобальных опций другой (например, нет `--root` и `--eprefix`).

**Разделы:** [Установка](#установка) · [Примеры использования](#примеры-использования) · [Вывод и глобальные опции](#вывод-и-глобальные-опции) · [Модули](#модули) · [Репозитории](#репозитории) · [Файлы](#файлы)

## Установка

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
<summary><h3>Вручную</h3></summary>

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

> **Зависимости:** `gnupg` (ключи для импорта репозиториев с базы) `pacman` (libalpm) `curl` (для `reflector`) `aura-emerge` (eselect правит /etc/portage/, без АЕ это бессмысленно)

## Примеры использования
```bash
# Обзор
eselect                                  # глобальные опции и список модулей
eselect profile                          # действия одного модуля
eselect profile help                     # то же, с описанием

# Профиль
eselect profile list                     # нумерованный список, текущий помечен *
eselect profile show                     # текущая ссылка make.profile
sudo eselect profile set 3               # по номеру из `list`
sudo eselect profile set default/linux/amd64/23.0   # или по пути

# Репозитории
eselect repository list                  # настроенные и известные репозитории
eselect repository list -i               # только настроенные
sudo eselect repository enable chaotic-aur   # из локальной базы + импорт ключей
sudo eselect repository add myrepo 'https://example.org/$repo/os/$arch' -p 60
sudo eselect repository disable myrepo   # блок остаётся, enable его восстановит
sudo eselect repository remove -f myrepo

# World
eselect world show
eselect world regen -p                   # что будет добавлено
sudo eselect world regen -a              # добавить недостающие, с подтверждением

# Список пакетов репозитория (чистый текст, можно писать в файл)
eselect repository packages core > packages
eselect repository packages -r -v extra | tee extra.list   # extra/foo 1.2-1

# Зеркала: ранжировать mirrorlist-ы репозиториев (самые быстрые — первыми)
eselect reflector list                   # какой mirrorlist у какого репозитория
eselect reflector -p core blackarch      # только показать рейтинг, ничего не писать
sudo eselect reflector core blackarch world system
sudo eselect reflector -n 5 --protocol all blackarch

# Java (встроенный archlinux-java)
eselect java status                      # окружения, текущее помечено (default)
sudo eselect java set java-21-openjdk
sudo eselect java fix                    # починить ссылку default

# Новости
eselect news                             # Arch News
eselect news read 8                      # прочитать новость
eselect news read all                    # пометить прочитанными все

# Кеши и блокировка базы
sudo eselect cache clean --all -p        # демонстрационный запуск
sudo eselect cache clean --pkgcache --build -y
eselect db status
sudo eselect db unlock                   # откажет, пока работает pacman/emerge
```

## Вывод и глобальные опции

Вывод повторяет Gentoo `eselect`: зелёные жирные заголовки списков, нумерованный столбец `[N]`, `*` отмечает активную запись, ошибки выглядят как `!!! Error: …`.

| Опция | Назначение |
|-------|------------|
| `--brief` | Короче вывод: без заголовков (списков и справки) и без маркера `*` |
| `--colour=yes\|no\|auto` | Включить/Выключить цвета |


## Модули

```
eselect <модуль> <действие> [аргументы]
eselect <модуль> help             # действия и опции модуля
eselect <модуль> <действие> --help  # man eselect-<модуль>-<действие>
```

| Модуль | Действия | Примечания |
|--------|----------|------------|
| **profile** | `list` `show` `set <N\|path>` | `/etc/portage/make.profile` → `/var/db/portage/profiles/…` |
| **repository** | `list [-i]` `enable` `disable` `remove [-f]` `add <name> <uri> [-p N] [--sig L]` `packages [-v] [-r] <name>` | `/etc/portage/repos.conf/eselect.conf` + импорт ключей и mirrorlist; `packages` читает sync-базу напрямую |
| **reflector** | `[-n N] [-t S] [-j N] [--protocol P] [--active-only] [-p] <repo>...` `list` | Ранжирует зеркала mirrorlist-ов; отдельное действие не нужно — достаточно имён репозиториев |
| **java** | `status` `get` `set <env>` `unset` `fix` | `/usr/lib/jvm/default`; доступно и как `archlinux-java <действие>` |
| **news** | `list` `read [N\|all]` | Делегирует в `emerge --news` |
| **world** | `show` `regen [-p] [-a]` | Добавляет пакеты из explicit, которых нет в world |
| **cache** | `clean [--pkgcache] [--sources] [--packages] [--build] [--all] [-p] [-y]` | Кеш pacman + `~/.cache/aura-emerge/` всех пользователей |
| **db** | `status` `lock` `unlock [-f]` | `/var/lib/pacman/db.lck` |

### profile

Профиль — это конечный каталог дерева `/var/db/portage/profiles`, в котором есть `packages`, `package.profile`, `package.mask`, `make.defaults` или `parent`.

`set` принимает номер из `list`, абсолютный путь или путь относительно дерева. Ссылка записывается относительной и подменяется атомарно.

Если `make.profile` — настоящий каталог или файл, `set` спросит и перенесёт его в `make.profile.bak` — никогда не удаляет. Как и оригинал, `set` при успехе ничего не выводит.

### reflector

Ранжирует зеркала репозиториев и переписывает их mirrorlist: самые быстрые идут первыми. Действие указывать не нужно: имена репозиториев сразу после модуля уже означают ранжирование — `eselect reflector core blackarch world system`.

Читается только тот источник, который действует: при `FEATURES="repos"` в `make.conf` — `/etc/portage/repos.conf` (`include =`), без него — `/etc/pacman.conf` (`Include =`). Репозитории, которые делят один файл (`core`, `extra`, `multilib` → `/etc/pacman.d/mirrorlist`), ранжируются один раз. У репозитория с единственным `sync-uri` или `Server` ранжировать нечего — это сообщается явно.

Кандидаты — все строки `Server =` файла, **включая закомментированные**. Каждое зеркало проверяется через `curl`: скачиваются первые 512 КиБ файла `<repo>.db`, и учитывается время. Для общего файла берётся первый из указанных репозиториев; `$repo`, `$arch`, `$arch_v3`, `$arch_v4` подставляются.

| Опция | Назначение |
|-------|------------|
| `-n, --number <N>` | Сколько самых быстрых зеркал оставить активными (10) |
| `-t, --timeout <S>` | Полное время проверки одного зеркала (подключение и загрузка), секунды (5) |
| `-j, --jobs <N>` | Параллельных проверок (8) |
| `--protocol <P>` | `https` (по умолчанию), `http` или `all` |
| `--active-only` | Учитывать только сейчас активные зеркала |
| `-p, --pretend` | Показать рейтинг, ничего не писать (root не нужен) |

Если сервер игнорирует диапазоны (`Range`), он отдаёт файл целиком, и большой `.db` может не уложиться в таймаут — такое зеркало попадёт в недоступные. `reflector list` показывает файл и число активных/всех зеркал для каждого репозитория.

### java

Встроенная замена `archlinux-java` (раскладка как у `java-environment-common`): окружение — каталог `/usr/lib/jvm/<env>` с исполняемым `bin/java`; если есть `<env>/jre/bin/java`, в списке появляется и `<env>/jre`.

| Действие | Что делает |
|----------|------------|
| `status` | Список окружений, текущее помечено `(default)` |
| `get` | Выводит имя текущего окружения (ошибка, если не задано) |
| `set <env>` | Делает окружение текущим: ссылки `default` и `default-runtime` (root) |
| `unset` | Удаляет обе ссылки (root) |
| `fix` | Если `default` не задан или сломан, выбирает окружение с наибольшим номером `java-N-…` (JDK/JRE-каталоги предпочтительнее `<env>/jre`); исправное не трогает, при успехе молчит (root) |

Если бинарник `eselect` доступен под именем `archlinux-java`, то `archlinux-java <действие>` равносильно `eselect java <действие>`, поэтому скрипты установки Java-пакетов продолжают работать. Ссылку `/usr/bin/archlinux-java` нужно создать самостоятельно: она конфликтует с пакетом `java-environment-common`.

### repository packages

`eselect repository packages [-v] [-r] <name>` читает `/var/lib/pacman/sync/<name>.db` напрямую (zstd, gzip или обычный tar; xz и bzip2 не поддерживаются), без `pacman` и `emerge`, и печатает по одному пакету в строке — только текст, без цветов и заголовков, чтобы вывод можно было направить в файл или `tee` (например, для создания профилей). `-v` добавляет версию, `-r` — префикс `<repo>/`. Если базы нет, команда подсказывает выполнить синхронизацию и перечисляет доступные репозитории.

### world

`show` выводит файл world. `regen` читает явно установленные пакеты напрямую из `/var/lib/pacman/local` (без вызова `pacman`) и дописывает те, которых в world нет. `-p` только показывает их и не требует root, `-a` спрашивает перед записью.

### cache

| Флаг | Что чистится |
|------|--------------|
| `--pkgcache` (`--pacman`) | `/var/cache/pacman/pkg` |
| `--sources` | `~/.cache/aura-emerge/sources` |
| `--packages` | `~/.cache/aura-emerge/packages` |
| `--build` | `~/.cache/aura-emerge/build` |
| `--all` | всё перечисленное |

Обрабатываются `/root` и каждый `/home/*`. Под root символические ссылки, подброшенные пользователем, не раскрываются: каждый компонент пути ниже домашнего каталога должен быть настоящим каталогом. `-p` — пробный запуск, `-y` — без подтверждения.

### db

`status` показывает блокировку, её возраст и запущенные менеджеры пакетов; `lock` создаёт `db.lck`; `unlock` удаляет его и отказывает, пока работает `pacman`, `emerge`, `paru`, `yay` или `pamac-daemon` (`-f` принудительно).

## Репозитории

AE читает `/etc/portage/repos.conf` (файл или `repos.conf/*.conf`, `FEATURES="repos"`). eselect владеет там ровно одним файлом:

```
/etc/portage/repos.conf/eselect.conf
```

* `enable` / `add` пишут блок `[name]` (ключи, которые понимает AE: `priority`, `sync-uri`, `include`, `verify-signature`, `location`, `auto-sync`, `sync-type`).
* `disable` оставляет блок строками с префиксом `#| ` (для AE это комментарии), поэтому `enable` может его восстановить.
* `remove` удаляет блок.
* Имя, уже определённое в другом файле repos.conf, даёт предупреждение и вопрос.
* Если `repos.conf` всё ещё один файл, eselect предлагает превратить его в каталог (старый файл становится `repos.conf/00-main.conf`).
* eselect предупреждает, если в `make.conf` не задан `FEATURES="repos"`.

`list` показывает настроенные репозитории (`*` — включён; `(disabled)`; `(repos.conf)` — определён в другом файле) и, без `-i`, известные, для которых есть ключи или шаблон.

### Ключи

При `eselect repository enable|add <name>` (первое совпадение выигрывает):

1. `/usr/share/pacman/keyrings/<name>.gpg` (+ `-trusted` / `-revoked`) — те же шаги, что и `pacman-key --populate <name>`
2. `/var/db/portage/repos/<name>/keys/<name>.gpg` (+ trusted/revoked)

Шаблон для `enable` необязателен: `/var/db/portage/repos/<name>/<name>.conf`. Рядом можно положить `mirrorlist`: если файл из `include =` шаблона ещё не существует, `enable` установит его оттуда; если шаблона нет, выводится предупреждение с подсказкой (`eselect reflector <name>`).

```
/var/db/portage/repos/chaotic/
  chaotic.conf                 # блок [chaotic], который копирует `enable`
  mirrorlist                   # необязательно: ставится в include-путь, если его нет
  keys/
    chaotic.gpg
    chaotic-trusted            # строки FPR:LEVEL: (gpg ownertrust)
    chaotic-revoked            # необязательно, по одному FPR в строке
```


## Файлы

| Путь | Назначение |
|------|------------|
| `/etc/portage/make.profile` | Выбранный профиль (символическая ссылка) |
| `/var/db/portage/profiles/` | Дерево профилей |
| `/etc/portage/repos.conf/eselect.conf` | Репозитории, которыми управляет eselect |
| `/var/db/portage/repos/<name>/` | Шаблон и ключи репозитория |
| `/usr/share/pacman/keyrings/` | Keyring-и (`<name>.gpg`) |
| `/etc/pacman.d/*mirrorlist*` | Mirrorlist-ы репозиториев (их переписывает `reflector`, копия `*.eselect-bak`) |
| `/var/lib/pacman/sync/<repo>.db` | Sync-базы (для `repository packages`) |
| `/var/lib/pacman/local/` | Локальная база (для `world regen`) |
| `/usr/lib/jvm/` | Окружения Java и ссылки `default`, `default-runtime` |
| `/etc/portage/world` | Файл world |
| `/etc/portage/make.conf` | Проверяется на `FEATURES="repos"` |
| `/var/lib/pacman/db.lck` | Блокировка базы |

## Автор / Лицензия

Copyright (C) 2026 **Undercat037**  
Лицензия GPL-3.0
