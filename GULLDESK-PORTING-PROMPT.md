# Промпт: полный порт Kernora shell → GullDesk на GullOS (Arch, Plasma 6)

Скопируй этот текст в агента. Заполни переменные в начале. Агент сначала составит план и дождётся подтверждения.

---

## 0. Переменные

- `DISTRO_NAME` = GullOS
- `ARCH_ROOT` = корень репозитория GullOS (где `pkgbuilds/`, `profile/`, `scripts/`)
- `WORKSPACE` = путь к клону Kernora-OS (по умолчанию `/home/nvrchg/projects/Kernora-OS-`)
- `PLASMA_VERSION` = целевая версия Plasma (например, "6.3" или "6.4")
- `KF_VERSION` = целевая версия KF (например, "6.13")
- `QT_VERSION` = целевая версия Qt (например, "6.8")
- `NEW_NAME` = GullDesk
- `OLD_NAME` = Kernora
- `OLD_LOWER` = kernora
- `NEW_LOWER` = gulldesk

---

## 1. Контекст GullOS (НЕ НАРУШАТЬ)

Из `AI_CONTEXT.md`:
- База: vanilla Arch Linux.
- Пакетирование: PKGBUILDs в `pkgbuilds/`.
- ISO: `mkarchiso` с профилем в `profile/`.
- VDI: `pacstrap` через `scripts/build-disk-image.sh`.
- Ядра: `linux-cachyos` (default) + `linux`.
- Графика: Wayland-only, compositor `labwc` + `seatd`. **Без X11.**
- Тема: Catppuccin Mocha токены (будет заменена/совмещена с Material 3).
- Rust workspace: `rust-workspace/` — **НЕ ТРОГАТЬ**.
- `gull-lock`: boot-first на установленных системах (маркер `/etc/gullos/installed`). Live без лока.
- Calamares installer запускается через `gull-install`.
- Сборка: cloud-only (нет локального cargo/make/archiso). LF endings. No X11.

**Цель:** заменить `labwc` на Plasma 6 + Kernora shell, переименовав Kernora → GullDesk. Сохранить gull-lock, gull-install, Rust workspace, cloud-only сборку, LF endings, no X11.

---

## 2. Роль и цель

Ты — senior инженер по портированию Linux desktop shells. Задача: **полностью перенести оболочку Kernora (KED 2) в GullOS**, заменив labwc на Plasma 6 + Kernora shell, с полным переименованием Kernora → GullDesk.

**Сначала составь детальный план, согласуй, и только после подтверждения начинай реализацию.**

---

## 3. Источник (Kernora-OS)

Репозиторий: `${WORKSPACE}`

Ключевые пути:
- Пакет оболочки: `${WORKSPACE}/packages/native/kernora-desktop`
- QML-оболочка: `${WORKSPACE}/packages/native/kernora-desktop/data/shell/`
- Панель: `${WORKSPACE}/packages/native/kernora-desktop/data/panel/`
- Системные поверхности: `${WORKSPACE}/packages/native/kernora-desktop/data/{lockscreen,notifications,osd,explorer,editmode,taskmanager,kwin-tabbox,look-and-feel}`
- Стиль Qt Quick: `${WORKSPACE}/packages/native/kernora-desktop/data/style/org.kernora.desktop/`
- Виджеты: `${WORKSPACE}/packages/native/kernora-desktop/data/widgets/`
- Обои: `${WORKSPACE}/packages/native/kernora-desktop/data/wallpaper/`
- Генераторы: `${WORKSPACE}/packages/native/kernora-desktop/tools/`
- Зависимости: `${WORKSPACE}/packages/native/kernora-apps`, `kernora-wallpapers`, `kwin-effect-rounded-corners`, `fonts-google-sans`
- Документация: `${WORKSPACE}/docs/DESKTOP.md`, `${WORKSPACE}/docs/SHELL-LOCATIONS.md`
- Сборка: `${WORKSPACE}/Makefile`, `${WORKSPACE}/packages/native/kernora-desktop/debian/rules`

Лицензии:
- Kernora own: GPL-2.0-or-later (репо GPL-3.0)
- Material Symbols + material_shapes.py: Apache-2.0
- KDE-адаптированные: upstream лицензии (SPDX)
- derived: upstream лицензии
- kernora-quickshare: GPL-3.0-or-later

---

## 4. Общие правила

1. **Сначала план.** Не пиши код до согласования. План: этапы, риски, зависимости, список пакетов, тест-план.
2. **Не трогай систему пользователя.** Всё в chroot/VM/контейнере или `${WORKSPACE}/_build`. Никаких изменений host.
3. **Сохраняй авторство и лицензии.** Не удаляй SPDX, copyright, NOTICE.
4. **Rename Kernora → GullDesk везде:** пакеты, пути, applet IDs (`org.kernora.*` → `org.gulldesk.*`), look-and-feel (`org.kernora.desktop` → `org.gulldesk.desktop`), color schemes (`KernoraDark` → `GullDeskDark`), plasma themes (`kernora` → `gulldesk`), икон-тема (`Kernora` → `GullDesk`), курсоры (`kernora_cursors` → `gulldesk_cursors`), обои (`org.kernora.*` → `org.gulldesk.*`), звуки (`kernora` → `gulldesk`), сервисы (`kernora-*` → `gulldesk-*`), kconf_update, skel-config, polkit, translations (.po domains), конфиги (`kernorarc` → `gulldeskc`? или сохранить? — решить в плане), пути (`/usr/share/kernora/` → `/usr/share/gulldesk/`).
   - **Важно:** rename должен быть консистентным. Все внутренние ссылки (QML imports, config keys, D-Bus service names, kglobalaccel) должны совпадать.
5. **Разбивка на пакеты** в `pkgbuilds/`:
   - `gulldesk-desktop-shell` — QML-оболочка, панель, lockscreen, notifications, OSD, explorer, editmode, taskmanager, tabbox, look-and-feel, splash, panel layout.
   - `gulldesk-desktop-themes` — цветовые схемы, Plasma theme, Kvantum, GTK, Aurorae, Qt Quick style, звуки.
   - `gulldesk-desktop-icons` — икон-тема GullDesk + Material Symbols.
   - `gulldesk-desktop-cursors` — курсоры.
   - `gulldesk-desktop-wallpapers` — живые обои + materialshapes.
   - `gulldesk-desktop-widgets` — desktop-виджеты.
   - `gulldesk-desktop-services` — systemd-юниты, kconf_update, скрипты, polkit, crash-notifier, panic-report.
   - `gulldesk-desktop-fonts` — шрифты (если не системные).
   - `gulldesk-apps` — порт `kernora-apps` (Appearance, MprisPlayer, AppIconSync, update-notifier) с rename.
   - `gulldesk-wallpapers` — порт `kernora-wallpapers`.
   - `kwin-effect-rounded-corners` — можно использовать upstream (уже есть в AUR) или портировать.
   - `fonts-google-sans` — порт или использовать AUR.
6. **Адаптация под GullOS:**
   - Plasma 6 вместо labwc: установить `plasma-desktop`, `plasma-workspace`, `kwin`, `sddm` (или сохранить автологин), `plasma-nm`, `bluedevil`, `powerdevil`, `kscreen`, `kded`, `kglobalacceld`, `kwin-x11`? Нет — Wayland-only: `kwin-wayland`.
   - Удалить из профиля: `labwc`, `seatd` (или оставить seatd для совместимости, но Plasma использует собственный).
   - Сохранить: `gull-lock` (адаптировать под Plasma session), `gull-install` (Calamares), Rust workspace.
   - `mkarchiso` профиль: добавить Plasma-пакеты, удалить labwc.
   - `scripts/build-disk-image.sh` (pacstrap): добавить Plasma-пакеты.
   - Cloud-only: сборка через CI (GitHub Actions или аналог), артефакты в releases.
   - LF endings, no X11: не добавлять X11-зависимости.
7. **Версии.** Проверять совместимость с `${PLASMA_VERSION}` / `${KF_VERSION}` / `${QT_VERSION}`. Если API изменился — адаптировать QML.
8. **Тесты.** Smoke-тест в VM: установка, запуск Plasma, проверка QML-ошибок, лаунчер, шторка, lock screen, OSD, уведомления, виджеты, обои, темы.

---

## 5. Этапы (по порядку, с отчётами)

### Этап A — Анализ и план
- Изучить `${WORKSPACE}/packages/native/kernora-desktop/debian/rules`, `debian/control`, `debian/*.preinst/postinst/triggers`.
- Составить полный список diverted-файлов.
- Составить список systemd-юнитов, kconf_update, autostart, skel-config.
- Составить список зависимостей и версий.
- Составить список генераторов и их входов/выходов.
- Составить полный список строк/имён для rename (grep по всему репо).
- Изучить GullOS профиль: `profile/`, `scripts/build-disk-image.sh`, `pkgbuilds/`.
- **Выдать план** с этапами, рисками, оценкой, списком пакетов, тест-планом. **Дождаться подтверждения.**

### Этап B — Сборка артефактов Kernora
- Собрать `kernora-desktop` в изолированной среде по `Makefile`/`debian/rules`.
- Сохранить артефакты в `${WORKSPACE}/_build/kernora-artifacts/`.
- Проверить все генераторы.
- Отчёт: список файлов, размеры, ошибки.

### Этап C — Rename Kernora → GullDesk
- Создать копию артефактов и исходников в рабочую директорию GullOS (например, `${ARCH_ROOT}/src/gulldesk/`).
- Выполнить глобальный rename:
  - Файлы/директории: `kernora` → `gulldesk`, `Kernora` → `GullDesk`, `KERNORA` → `GULLDESK`.
  - Содержимое: все строки `kernora`/`Kernora`/`KERNORA` → `gulldesk`/`GullDesk`/`GULLDESK` (кроме лицензий/авторства, где нужно сохранить оригинальные имена? — решить в плане; для upstream KDE файлов сохранить авторство).
  - Applet IDs: `org.kernora.*` → `org.gulldesk.*`.
  - Look-and-feel: `org.kernora.desktop` → `org.gulldesk.desktop`.
  - Color schemes: `Kernora*` → `GullDesk*`.
  - Plasma themes: `kernora*` → `gulldesk*`.
  - Икон-тема: `Kernora` → `GullDesk`, `Kernora-<colour>` → `GullDesk-<colour>`.
  - Курсоры: `kernora_cursors` → `gulldesk_cursors`.
  - Обои: `org.kernora.*` → `org.gulldesk.*`.
  - Звуки: `kernora` → `gulldesk`.
  - Сервисы: `kernora-*` → `gulldesk-*`.
  - kconf_update скрипты: rename.
  - Переводы: `.po` domains `plasma_applet_org.kernora.*` → `plasma_applet_org.gulldesk.*`.
  - Конфиги: `kernorarc` → `gulldeskc` (или сохранить? — решить; лучше переименовать для чистоты, но проверить все ссылки).
  - Пути: `/usr/share/kernora/` → `/usr/share/gulldesk/`, `/usr/libexec/kernora-*` → `/usr/libexec/gulldesk-*`.
- Проверить, что rename не сломал импорты/ссылки (grep по старым именам).
- Отчёт: список переименованных файлов/строк, оставшиеся ссылки на старые имена.

### Этап D — Создание структуры пакетов в GullOS
- В `${ARCH_ROOT}/pkgbuilds/` создать директории для каждого пакета из п.4.
- В каждой: `PKGBUILD`, `*.install` (post_install/pre_remove), при необходимости хуки.
- Настроить сборку: копирование переименованных артефактов + статических файлов из `data/`.
- Адаптировать зависимости для Arch (имена пакетов: `plasma-desktop`, `plasma-workspace`, `kwin`, `qt6-base`, `qt6-declarative`, `qt6-quickcontrols2`, `kf6-*`, `kvantum`, `breeze-gtk`, `noto-fonts` и т.д.).
- Отчёт: дерево директорий, PKGBUILD-ы.

### Этап E — Адаптация профиля и скриптов GullOS
- `profile/` (mkarchiso): добавить Plasma-пакеты, удалить labwc/seatd (или оставить seatd), добавить SDDM (или автологин), настроить session.
- `scripts/build-disk-image.sh` (pacstrap): добавить Plasma-пакеты.
- Адаптировать `gull-lock`: boot-first под Plasma (SDDM autologin → Plasma session, или ksmserver). Маркер `/etc/gullos/installed`.
- Адаптировать `gull-install`: Calamares уже используется в Kernora; настроить под GullOS (брендинг GullDesk, модули).
- Cloud-only: настроить CI (GitHub Actions) для сборки пакетов и ISO.
- Отчёт: изменения в профиле/скриптах.

### Этап F — Адаптация дивертов и системных поверхностей
- Заменить `dpkg-divert` на pacman-оверлей/конфликты.
- Таблица: источник (GullDesk) → цель (путь) → владелец на Arch → статус.
- Pacman-хуки для обновления кэшей.
- Отчёт: таблица дивертов.

### Этап G — Адаптация сервисов и конфигов
- Перенести systemd-юниты, kconf_update, autostart, skel-config, polkit.
- Адаптировать пути.
- Настроить kded-модули и kconf_update.
- Отчёт: список сервисов, статусы.

### Этап H — Адаптация зависимостей и опциональных компонентов
- Портировать `kernora-apps` → `gulldesk-apps` (Appearance, MprisPlayer, AppIconSync, update-notifier) с rename.
- Портировать `kernora-wallpapers` → `gulldesk-wallpapers`.
- `kwin-effect-rounded-corners`: использовать upstream AUR или портировать.
- `fonts-google-sans`: использовать AUR или портировать.
- Если что-то не портировано — вырезать и отключить фичи, отчёт.
- Отчёт: что вырезано/адаптировано.

### Этап I — Тестирование
- Собрать пакеты в chroot/VM (cloud-only: CI).
- Установить в тестовую среду с Plasma `${PLASMA_VERSION}`.
- Запустить Plasma, проверить журнал на QML-ошибки.
- Проверить: лаунчер (Meta), шторка (Meta+A), clipboard (Meta+V), Alt+Tab, lock screen, OSD, уведомления, виджеты, обои, темы, gull-lock, gull-install.
- Зафиксировать регрессии и исправить.
- Отчёт: результаты тестов, баги, фиксы.

### Этап J — Финализация и документация
- Обновить документацию GullOS (добавить раздел про GullDesk).
- Добавить README для пакетов.
- Подготовить релиз-артефакты (pkg.tar.zst, ISO).
- Отчёт: финальная структура, инструкции.

---

## 6. Критерии приёмки (Definition of Done)

- [ ] Все пакеты собираются без ошибок в чистой среде (cloud-only CI).
- [ ] ISO собирается через mkarchiso.
- [ ] VDI собирается через pacstrap.
- [ ] Установка в тестовую среду не ломает систему.
- [ ] Plasma запускается без QML-ошибок (или только некритичные).
- [ ] Launcher, quick settings, clipboard, Alt+Tab, lock screen, OSD, уведомления работают.
- [ ] Темы (цвета, иконки, курсоры, Kvantum, GTK, Aurorae) применяются.
- [ ] Живые обои работают (хотя бы Shapes).
- [ ] Виджеты добавляются и работают.
- [ ] gull-lock работает (boot-first на установленной системе).
- [ ] gull-install (Calamares) работает.
- [ ] Rust workspace не изменён.
- [ ] LF endings, no X11.
- [ ] Rename GullDesk консистентен (нет ссылок на старые имена).
- [ ] Лицензии и авторство сохранены.
- [ ] Документация обновлена.

---

## 7. Формат отчётов

После каждого этапа:
- **Что сделано** (кратко)
- **Артефакты** (пути)
- **Проблемы/риски**
- **Следующий шаг** (что нужно от меня)

Не пропускай этапы. Не начинай реализацию без подтверждения плана.

---

## 8. Дополнительные указания

- Если версия Plasma новее 6.3 — будь готов к поломкам QML API. Проверяй импорты.
- Не используй Debian-специфичные пути/команды на целевой системе.
- Все пути по FHS (Arch): `/usr/bin`, `/usr/lib`, `/usr/share`, `/etc`.
- Используй `namcap` для проверки PKGBUILD-ов.
- Cloud-only: не запускай `make`/`cargo`/`mkarchiso` локально; используй CI или контейнеры.
- LF endings: убедись, что все файлы используют LF.
- No X11: не добавляй X11-зависимости; используй Wayland-only компоненты.
- Rust workspace: не изменяй.
- При ошибках — анализируй логи, не пропускай.

---

**Начни с Этапа A: изучи репозитории (Kernora-OS и GullOS) и выдай план. Жди подтверждения.**
