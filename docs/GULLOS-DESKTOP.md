# GullDesk on GullOS

GullOS now ships Plasma 6 (Wayland-only) with the GullDesk shell (renamed from
Kernora KED 2). Labwc/seatd have been removed.

## Packages
- gulldesk-desktop-shell — QML shell, panel, lockscreen, OSD, notifications, edit mode, diverted surfaces.
- gulldesk-desktop-themes — color schemes, Plasma theme, Kvantum, GTK, Aurorae, Qt Quick style, sounds, grub/plymouth.
- gulldesk-desktop-icons — GullDesk icon theme + Material Symbols.
- gulldesk-desktop-cursors — cursor theme.
- gulldesk-desktop-wallpapers — live wallpapers (Shapes, Rhythm, Starfall, Pastel, Ribbon).
- gulldesk-desktop-widgets — desktop widgets.
- gulldesk-desktop-services — systemd units, kconf_update, polkit, crash/panic reporters.
- gulldesk-apps — GullDesk applications (settings, files, editor, photos, player, calc, screenshot, terminal, welcome, store, update-notifier, appearance, app-icon-sync, mpris).
- gulldesk-wallpapers — picture wallpapers.
- calamares-settings-gullos — installer config + GullDesk branding.

## Boot
- gull-session launches `startplasma-wayland`.
- Boot-first lock on installed systems via Plasma lock screen (GullDesk LockScreen.qml) + getty autologin.
- gull-lock binary retained for compatibility.

## Build
Cloud-only via GitHub Actions (build-os.yml): rust-check → build-packages → build-iso/build-vdi → release.
