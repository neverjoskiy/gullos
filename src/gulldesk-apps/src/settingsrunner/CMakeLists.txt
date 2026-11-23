# SPDX-FileCopyrightText: 2026 GullDesk Developers
# SPDX-License-Identifier: GPL-2.0-or-later

add_executable(gulldesk-settings-runner main.cpp)
target_link_libraries(gulldesk-settings-runner PRIVATE Qt6::Gui Qt6::DBus Qt6::Qml KF6::I18n KF6::I18nQml)
target_compile_definitions(gulldesk-settings-runner PRIVATE TRANSLATION_DOMAIN="gulldesk-settings")
install(TARGETS gulldesk-settings-runner DESTINATION ${KDE_INSTALL_LIBEXECDIR})

configure_file(org.gulldesk.settings.runner.service.in ${CMAKE_CURRENT_BINARY_DIR}/org.gulldesk.settings.runner.service @ONLY)
install(FILES ${CMAKE_CURRENT_BINARY_DIR}/org.gulldesk.settings.runner.service DESTINATION ${KDE_INSTALL_DBUSSERVICEDIR})
install(FILES org.gulldesk.settings.desktop DESTINATION ${KDE_INSTALL_DATADIR}/krunner/dbusplugins)
