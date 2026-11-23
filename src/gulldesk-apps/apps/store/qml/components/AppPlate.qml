/*
    SPDX-FileCopyrightText: 2026 GullDesk Developers
    SPDX-License-Identifier: GPL-2.0-or-later
*/

import QtQuick
import org.gulldesk.m3
import org.gulldesk.ui

Rectangle {
    id: root

    property int size: 96
    property string iconName

    implicitWidth: size
    implicitHeight: size
    width: size
    height: size
    radius: M3.classic ? Math.round(size * 0.12) : Math.round(size * 0.24)
    color: M3.surfaceContainerHigh

    AppIcon {
        anchors.centerIn: parent
        size: Math.round(root.size * 0.74)
        name: root.iconName
        roundToIconSize: false
    }
}
