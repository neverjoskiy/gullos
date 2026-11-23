# SPDX-FileCopyrightText: 2026 GullDesk Developers
# SPDX-License-Identifier: GPL-2.0-or-later

add_library(gulldesk-appicons STATIC appicons.cpp appicons.h appiconsync.cpp appiconsync.h)
target_link_libraries(gulldesk-appicons PUBLIC Qt6::Gui Qt6::DBus KF6::ConfigCore KF6::Service)

add_executable(gulldesk-update-notifier main.cpp avatarsync.cpp avatarsync.h greetersync.cpp greetersync.h homeiconsync.cpp homeiconsync.h)
target_link_libraries(gulldesk-update-notifier PRIVATE
    gulldesk-appicons
    Qt6::Gui Qt6::DBus Qt6::Network
    KF6::ConfigCore KF6::CoreAddons KF6::DBusAddons KF6::I18n KF6::Notifications KF6::Service
    PK::packagekitqt6)
target_compile_definitions(gulldesk-update-notifier PRIVATE
    TRANSLATION_DOMAIN="gulldesk-update-notifier"
    GULLDESK_APPS_VERSION="${PROJECT_VERSION}")
install(TARGETS gulldesk-update-notifier ${KDE_INSTALL_TARGETS_DEFAULT_ARGS})

add_executable(gulldesk-app-icons appiconstool.cpp)
target_link_libraries(gulldesk-app-icons PRIVATE gulldesk-appicons)
install(TARGETS gulldesk-app-icons ${KDE_INSTALL_TARGETS_DEFAULT_ARGS})
