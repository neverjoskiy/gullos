# SPDX-FileCopyrightText: 2026 GullDesk Developers
# SPDX-License-Identifier: GPL-2.0-or-later

add_executable(gulldesk-setup-apply main.cpp)
target_link_libraries(gulldesk-setup-apply PRIVATE Qt6::Gui Qt6::Qml Qt6::DBus KF6::ConfigCore)
install(TARGETS gulldesk-setup-apply DESTINATION libexec)
