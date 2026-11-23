# SPDX-FileCopyrightText: 2026 GullDesk Developers
# SPDX-License-Identifier: GPL-2.0-or-later

ecm_add_qml_module(gulldeskshell URI "org.gulldesk.shell" VERSION 1.0 GENERATE_PLUGIN_SOURCE)

target_sources(gulldeskshell PRIVATE
    appletsettings.cpp appletsettings.h
    windowblur.cpp windowblur.h
)

target_link_libraries(gulldeskshell PRIVATE
    Qt6::Core Qt6::Gui Qt6::Qml Qt6::Quick
    Plasma::Plasma Plasma::PlasmaQuick
    KF6::WindowSystem
    KF6::ConfigCore
)

ecm_finalize_qml_module(gulldeskshell DESTINATION ${KDE_INSTALL_QMLDIR})
