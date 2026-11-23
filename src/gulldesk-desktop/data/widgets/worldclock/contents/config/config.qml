/*
    SPDX-FileCopyrightText: 2026 GullDesk Developers
    SPDX-License-Identifier: GPL-2.0-or-later
*/

import QtQuick
import org.kde.plasma.configuration

ConfigModel {
    ConfigCategory {
        name: i18nd("plasma_applet_org.gulldesk.widget.worldclock", "City")
        icon: "globe"
        source: "ConfigCity.qml"
    }
}
