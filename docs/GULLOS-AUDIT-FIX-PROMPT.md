# Промпт: аудит и исправление ошибок порта GullDesk на GullOS

Скопируй этот текст в агента. Агент сначала проведёт аудит, составит план исправлений и дождётся подтверждения, затем исправит. **Не менять код до подтверждения плана.**

---

## 0. Контекст

- Репозиторий: `/home/nvrchg/projects/gullos` (GullOS, Arch-based)
- Источник: `/home/nvrchg/projects/Kernora-OS-` (Kernora, Debian-based)
- Порт уже выполнен (коммит `e9941b0` + 66 фиксов). Нужно найти оставшиеся ошибки/недоделки и исправить.
- Правила GullOS: cloud-only builds, LF endings, no X11, Rust workspace не трогать, labwc удалён, Plasma 6 + GullDesk.

---

## 1. Роль

Ты — senior инженер-аудитор. Задача: **найти все ошибки, недоделки и риски в порте GullDesk**, составить план исправлений, получить подтверждение, затем исправить. **Не менять код до подтверждения плана.**

---

## 2. Этап 1 — Аудит (только чтение, без изменений)

Проведи полный аудит репозитория. Проверь следующие категории:

### 2.1 Rename-консистентность
- `grep -rni "kernora" . --include='*'` (исключая .git, generated, и лицензии/SPDX) — найди все оставшиеся ссылки.
- Проверь `org.kernora`, `Kernora`, `KERNORA`, `kernorarc`, `kernora-*` в src/, profile/, scripts/, pkgbuilds/, docs/.
- Проверь, что `sddm/kernora` vs `sddm/gulldesk` согласованы в PKGBUILDs и generated/.
- Проверь `gulldeskc` vs `kernorarc` — все ссылки обновлены?

### 2.2 Пакеты (pkgbuilds/)
- Все ли пакеты из плана существуют? (shell, themes, icons, cursors, wallpapers, widgets, services, apps, wallpapers(picture), fonts, lock, sh, term, calamares-settings)
- Пустые/заглушечные PKGBUILDs (например, `gulldesk-desktop-fonts`) — найди.
- Зависимости: все ли Arch-имена корректны? (kf6-* → karchive, kconfig, и т.д.)
- `makedepends` достаточно? (rsvg-convert, python, grub, gettext, make, gcc, cmake, ecm, librsvg)
- `options=('!strip')` — ок?
- `source=()` — ок для локальных сборок?
- `install`-файлы и хуки — присутствуют ли?

### 2.3 Сборка (scripts/build.sh, debian/rules)
- `make -f debian/rules override_dh_auto_build` — вызывается в build(). Работает ли без debhelper? (dh не установлен в CI)
- `fetch-symbols --check` вызывается с `|| true` — ошибки игнорируются. Это риск.
- `generated/` gitignored — генерируется ли в CI? Проверь, что build() создаёт все нужные поддиректории.
- `compgen -G` и `cp -a` — выживут ли при отсутствии файлов?
- `nullglob` — включён ли в package()?

### 2.4 Профиль (profile/)
- `packages.x86_64` — есть ли labwc/seatd? (должны быть удалены)
- `gull-session` — корректно ли запускает Plasma? (XDG_CURRENT_DESKTOP=KDE, startplasma-wayland)
- `gull-autostart` — корректно ли работает с Plasma? (qdbus lock)
- Остались ли labwc configs в `profile/airootfs/home/gull/.config/labwc/`? (мусор)
- `customize_airootfs.sh` — актуален?

### 2.5 Сервисы и конфиги
- systemd-юниты: все ли перенесены? (gulldesk-* вместо kernora-*)
- kconf_update: скрипты и .upd файлы — все ли переименованы?
- polkit rules: пути корректны?
- skel-config: все ли файлы перенесены?
- autostart: актуален?

### 2.6 Диверты (docs/GULLDESK-DIVERTS.md)
- Все ли diverted файлы реально перенесены в PKGBUILD (install loops)?
- KCM pickers: портированы ли? (kcm_lookandfeel, kcm_colors, и т.д.)
- `dpkg-divert` → pacman: корректна ли стратегия оверлея?

### 2.7 Зависимости и отсутствующие компоненты
- `kwin-effect-rounded-corners` — есть ли pkgbuild или AUR-зависимость?
- `fonts-google-sans` — есть ли pkgbuild или AUR-зависимость?
- `gulldesk-desktop-fonts` — функционален ли?
- `gull-lock` — адаптирован ли под Plasma? (или оставлен Catppuccin)

### 2.8 CI (build-os.yml)
- Все ли пакеты в списке build.sh? (gulldesk-desktop-shell, themes, icons, cursors, wallpapers, widgets, services, apps, wallpapers(picture))
- `gulldesk-apps` пропускается, если нет KF6Screen dev — ок?
- build-iso: pacstrap wrapper с `--overwrite='*'` — корректно ли?
- build-vdi: pacstrap — корректно ли?

### 2.9 Документация
- `docs/GULLOS-DESKTOP.md` — актуальна ли?
- `docs/GULLDESK-DIVERTS.md` — актуальна ли?
- README — упоминает ли GullDesk?

### 2.10 Rust workspace
- Не изменён ли `rust-workspace/`? (должен быть нетронут)

---

## 3. Этап 2 — Составление плана

На основе аудита составь **детальный план исправлений** с:
- Списком найденных проблем (по категориям)
- Приоритетом (критично / важно / желательно)
- Оценкой трудоёмкости
- Порядком исправлений
- Тест-планом

**Дождись подтверждения плана. Не начинай исправления.**

---

## 4. Этап 3 — Исправления (после подтверждения)

Исправляй по плану. После каждого исправления — отчёт:
- Что исправлено
- Файлы
- Проверка (grep/build/test)

### Известные проблемы (для начала, но агент должен найти все сам):
1. `sddm/kernora` → `sddm/gulldesk` в 7 PKGBUILDs
2. Пустой `gulldesk-desktop-fonts` PKGBUILD
3. Отсутствующие `kwin-effect-rounded-corners` и `fonts-google-sans` pkgbuilds
4. Не портированные KCM pickers
5. labwc configs мусор
6. `fetch-symbols --check` игнорирует ошибки (`|| true`)
7. kwin-effect-rounded-corners зависимость в debian/control, но нет в Arch

---

## 5. Этап 4 — Верификация

- `grep -rni "kernora" src/ profile/ scripts/ pkgbuilds/` = 0 (кроме лицензий/SPDX)
- `bash -n` для всех скриптов
- `namcap` для PKGBUILDs
- Build packages в CI (dry-run)
- VM smoke-тест (если возможно)

---

## 6. Формат отчётов

После каждого этапа:
- **Что сделано**
- **Артефакты**
- **Проблемы/риски**
- **Следующий шаг**

Не пропускай этапы. Не начинай исправления без подтверждения плана.

---

**Начни с Этапа 1: проведи аудит и выдай список найденных проблем. Жди подтверждения.**
