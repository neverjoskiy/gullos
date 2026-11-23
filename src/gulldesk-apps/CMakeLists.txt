# SPDX-FileCopyrightText: 2026 GullDesk Developers
# SPDX-License-Identifier: GPL-2.0-or-later

cmake_minimum_required(VERSION 3.25)
project(gulldesk-apps VERSION 1.1.97 LANGUAGES CXX)

set(QT_MIN_VERSION 6.8)
set(KF_MIN_VERSION 6.13)

find_package(ECM ${KF_MIN_VERSION} REQUIRED NO_MODULE)
set(CMAKE_MODULE_PATH ${ECM_MODULE_PATH})

include(KDEInstallDirs)
include(KDECMakeSettings)
include(KDECompilerSettings NO_POLICY_SCOPE)
include(ECMQmlModule)
include(FeatureSummary)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(Qt6 ${QT_MIN_VERSION} REQUIRED COMPONENTS
    Core Gui Qml Quick QuickControls2 QuickWidgets Widgets DBus Network Svg)
find_package(KF6 ${KF_MIN_VERSION} REQUIRED COMPONENTS
    Archive Config CoreAddons DBusAddons I18n KIO Notifications Service WindowSystem)
find_package(KF6Screen REQUIRED)
find_package(PackageKitQt6 REQUIRED)
find_package(Plasma REQUIRED)
find_package(PlasmaQuick REQUIRED)
find_package(PkgConfig REQUIRED)
pkg_check_modules(LIBCRYPT REQUIRED IMPORTED_TARGET libcrypt)
pkg_check_modules(FLATPAK REQUIRED IMPORTED_TARGET flatpak)
find_package(AppStreamQt 1.0 REQUIRED)

add_subdirectory(src/system)
add_subdirectory(src/shell)
add_subdirectory(src/store)
add_subdirectory(src/apps)
add_subdirectory(src/filechooser)
add_subdirectory(src/updatenotifier)
add_subdirectory(src/settingsrunner)
add_subdirectory(src/terminal)
add_subdirectory(src/setupapply)

set(GULLDESK_APPS settings files welcome editor photos player calculator screenshot store share taskmanager drivers apkinstaller appsplash androiddialog appgroup)
foreach(app ${GULLDESK_APPS})
    install(DIRECTORY apps/${app}/qml/ DESTINATION ${KDE_INSTALL_DATADIR}/gulldesk/${app})
    install(FILES apps/${app}/org.gulldesk.${app}.desktop DESTINATION ${KDE_INSTALL_APPDIR})
endforeach()

install(DIRECTORY apps/filechooser/qml/ DESTINATION ${KDE_INSTALL_DATADIR}/gulldesk/filechooser)
install(DIRECTORY apps/terminal/qml/ DESTINATION ${KDE_INSTALL_DATADIR}/gulldesk/terminal)
install(DIRECTORY apps/terminal/schemes/ DESTINATION ${KDE_INSTALL_DATADIR}/gulldesk/terminal/schemes)
install(PROGRAMS apps/terminal/system/gulldesk-terminal-ports apps/terminal/system/gulldesk-terminal-share DESTINATION libexec)
install(FILES apps/terminal/system/org.gulldesk.terminal.policy DESTINATION ${KDE_INSTALL_DATADIR}/polkit-1/actions)
install(FILES apps/terminal/system/gulldesk-terminal.notifyrc DESTINATION ${KDE_INSTALL_KNOTIFYRCDIR})
install(FILES apps/taskmanager/org.gulldesk.taskmanager.desktop DESTINATION ${KDE_INSTALL_DATADIR}/kglobalaccel)

install(FILES apps/welcome/org.gulldesk.welcome-autostart.desktop
        DESTINATION ${KDE_INSTALL_AUTOSTARTDIR} RENAME org.gulldesk.welcome.desktop)
install(PROGRAMS data/systemsettings DESTINATION ${KDE_INSTALL_BINDIR})
install(FILES data/copyright/librewolf DESTINATION ${KDE_INSTALL_DATADIR}/gulldesk/copyright)
install(FILES data/knotifications/gulldesk-screenshot.notifyrc data/knotifications/gulldesk-updates.notifyrc data/knotifications/gulldesk-store.notifyrc
        DESTINATION ${KDE_INSTALL_KNOTIFYRCDIR})
install(FILES data/autostart/org.gulldesk.update-notifier.desktop data/autostart/org.gulldesk.setup-apply.desktop DESTINATION ${KDE_INSTALL_AUTOSTARTDIR})
install(FILES data/systemd/gulldesk-setup-style.service DESTINATION lib/systemd/user)
install(FILES data/whatsnew.json DESTINATION ${KDE_INSTALL_DATADIR}/gulldesk)
install(FILES data/store/featured.json DESTINATION ${KDE_INSTALL_DATADIR}/gulldesk/store)
install(FILES data/store/flathub.flatpakrepo DESTINATION ${KDE_INSTALL_DATADIR}/flatpak/remotes.d)
install(FILES data/store/60gulldesk-store DESTINATION ${KDE_INSTALL_SYSCONFDIR}/apt/apt.conf.d)

ki18n_install(po)

feature_summary(WHAT ALL FATAL_ON_MISSING_REQUIRED_PACKAGES)
