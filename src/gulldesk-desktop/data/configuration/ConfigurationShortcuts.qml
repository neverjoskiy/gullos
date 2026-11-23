/*
    SPDX-FileCopyrightText: 2013 Marco Martin <mart@kde.org>
    SPDX-FileCopyrightText: 2026 GullDesk Developers
    SPDX-License-Identifier: GPL-2.0-or-later
*/

import QtQuick
import QtQuick.Layouts
import org.kde.kquickcontrols
import org.kde.plasma.plasmoid
import org.kde.kcmutils as KCM
import org.gulldesk.m3
import org.gulldesk.ui as UI

KCM.SimpleKCM {
    id: root

    title: i18n("Shortcuts")

    signal configurationChanged

    property var keySequence: Plasmoid.globalShortcut

    function saveConfig() {
        Plasmoid.globalShortcut = root.keySequence
    }

    ColumnLayout {
        spacing: 20

        UI.Group {
            Layout.fillWidth: true
            note: i18nd("plasma_shell_org.kde.plasma.desktop", "This shortcut will activate the applet as though it had been clicked.")

            UI.ListItem {
                icon: "preferences-desktop-keyboard"
                title: i18nd("gulldesk-ui", "Shortcut")
                interactive: false

                trailing: Component {
                    KeySequenceItem {
                        keySequence: root.keySequence
                        modifierOnlyAllowed: true
                        onCaptureFinished: {
                            if (keySequence !== root.keySequence) {
                                root.keySequence = keySequence;
                                root.configurationChanged();
                            }
                        }
                    }
                }
            }
        }
    }
}
