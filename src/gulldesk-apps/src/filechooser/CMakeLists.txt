# SPDX-FileCopyrightText: 2026 GullDesk Developers
# SPDX-License-Identifier: GPL-2.0-or-later

add_executable(gulldesk-filechooser main.cpp portal.cpp portal.h types.cpp types.h)
target_link_libraries(gulldesk-filechooser PRIVATE
    Qt6::Widgets Qt6::DBus Qt6::Qml Qt6::Quick Qt6::QuickControls2
    KF6::CoreAddons KF6::I18n KF6::I18nQml KF6::WindowSystem)
target_compile_definitions(gulldesk-filechooser PRIVATE
    TRANSLATION_DOMAIN="gulldesk-files" GULLDESK_APPS_VERSION="${PROJECT_VERSION}")
install(TARGETS gulldesk-filechooser DESTINATION ${KDE_INSTALL_LIBEXECDIR})

configure_file(org.freedesktop.impl.portal.desktop.gulldesk.service.in
               ${CMAKE_CURRENT_BINARY_DIR}/org.freedesktop.impl.portal.desktop.gulldesk.service @ONLY)
install(FILES ${CMAKE_CURRENT_BINARY_DIR}/org.freedesktop.impl.portal.desktop.gulldesk.service
        DESTINATION ${KDE_INSTALL_DBUSSERVICEDIR})
install(FILES gulldesk.portal DESTINATION ${KDE_INSTALL_DATADIR}/xdg-desktop-portal/portals)
install(FILES kde-portals.conf DESTINATION ${KDE_INSTALL_SYSCONFDIR}/xdg-desktop-portal)
install(FILES org.gulldesk.filechooser.desktop DESTINATION ${KDE_INSTALL_APPDIR})
install(FILES kwinrulesrc DESTINATION ${KDE_INSTALL_SYSCONFDIR}/xdg)
install(FILES gulldesk-file-dialogs.sh DESTINATION ${KDE_INSTALL_SYSCONFDIR}/xdg/plasma-workspace/env)
