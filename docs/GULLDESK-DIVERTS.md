# GullDesk diverted system surfaces (Plasma 6 overlay)

On Debian these are installed via `dpkg-divert` (see `src/gulldesk-desktop/debian/gulldesk-desktop.preinst`).
On Arch (pacman) they are installed by `gulldesk-desktop-shell` directly over the
upstream Plasma files; the original `.kde` backups are not kept. Upgrading Plasma
may overwrite these files — re-install `gulldesk-desktop-shell` after a Plasma update.

## Diverted QML / system files

| GullDesk source | Target path (Arch) | Upstream owner | Status |
|---|---|---|---|
| data/lockscreen/LockScreen.qml | /usr/share/plasma/shells/org.kde.plasma.desktop/contents/lockscreen/LockScreen.qml | plasma-workspace | overlay |
| data/osd/Osd.qml | .../contents/osd/Osd.qml | plasma-workspace | overlay |
| data/configuration/AppletConfiguration.qml | .../contents/configuration/AppletConfiguration.qml | plasma-workspace | overlay |
| data/configuration/ConfigurationContainmentAppearance.qml | .../contents/configuration/ConfigurationContainmentAppearance.qml | plasma-workspace | overlay |
| data/configuration/ConfigurationContainmentActions.qml | .../contents/configuration/ConfigurationContainmentActions.qml | plasma-workspace | overlay |
| data/configuration/AboutPlugin.qml | .../contents/configuration/AboutPlugin.qml | plasma-workspace | overlay |
| data/configuration/ConfigurationShortcuts.qml | .../contents/configuration/ConfigurationShortcuts.qml | plasma-workspace | overlay |
| generated/explorer/* | .../contents/explorer/* | plasma-workspace | overlay |
| data/editmode/DesktopEditMode.qml | .../contents/views/DesktopEditMode.qml | plasma-workspace | overlay |
| data/panel/Panel.qml | .../contents/views/Panel.qml | plasma-workspace | overlay |
| data/configuration/panelconfiguration/PanelRepresentation.qml | .../contents/configuration/panelconfiguration/PanelRepresentation.qml | plasma-workspace | overlay |
| data/folderview/Config*.qml | /usr/share/plasma/plasmoids/org.kde.desktopcontainment/contents/ui/Config*.qml | plasma-desktop | overlay |
| data/folderview/FolderView.qml | .../org.kde.desktopcontainment/contents/ui/FolderView.qml | plasma-desktop | overlay |
| data/panelspacer/main.qml | /usr/share/plasma/plasmoids/org.kde.plasma.panelspacer/contents/ui/main.qml | plasma-desktop | overlay |
| data/systemtray/main.qml + items/* | /usr/share/plasma/plasmoids/org.kde.plasma.private.systemtray/contents/ui/... | plasma-workspace | overlay |
| data/notifications/main.qml + kernora/ | /usr/share/plasma/plasmoids/org.kde.plasma.notifications/contents/ui/... | plasma-workspace | overlay |
| data/taskmanager/*.qml | /usr/share/plasma/plasmoids/org.kde.plasma.taskmanager/contents/ui/... | plasma-workspace | overlay |
| data/systemmonitor/main.qml + main.xml | /usr/share/plasma/plasmoids/org.kde.plasma.systemmonitor/contents/... | plasma-workspace | overlay |
| data/panel/layout.js | /usr/share/plasma/layout-templates/org.kde.plasma.desktop.defaultPanel/contents/layout.js | plasma-workspace | overlay |
| /usr/share/kglobalaccel/org.kde.krunner.desktop | diverted | kglobalaccel | overlay |
| /usr/share/kglobalaccel/org.kde.plasma.emojier.desktop | diverted | kglobalaccel | overlay |
| /usr/share/plymouth/plymouthd.defaults | diverted | plymouth | overlay |

## Diverted KCM pickers (replaced by GullDesk versions)
kcm_lookandfeel, kcm_desktoptheme, kcm_colors, kcm_style, kcm_icons, kcm_cursortheme,
kcm_kwindecoration, kcm_splashscreen, kcm_sddm, kcm_soundtheme, kcm_landingpage,
kcm_breezedecoration, kcm_kwintabbox (so + desktop).

## Diverted binaries
/usr/bin/lookandfeeltool, /usr/bin/plasma-apply-lookandfeel, /usr/bin/kvantummanager,
/usr/share/applications/kvantummanager.desktop, /usr/bin/waydroid (+ waydroid desktops).

## Pacman notes
- No native `dpkg-divert`; overlay is best-effort. After Plasma package upgrades, re-install `gulldesk-desktop-shell`.
- `backup=()` not used (too many files); document manual recovery via pacman cache if needed.
