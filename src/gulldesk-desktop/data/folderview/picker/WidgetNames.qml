/*
    SPDX-FileCopyrightText: 2026 GullDesk Developers
    SPDX-License-Identifier: GPL-2.0-or-later
*/

import QtQml

QtObject {
    readonly property var apps: ({
        "battery": i18nd("gulldesk-ui", "Battery"),
        "calendar": i18nd("gulldesk-ui", "Calendar"),
        "clock": i18nd("gulldesk-ui", "Clock"),
        "gulldesk": "GullDesk",
        "player": i18nd("gulldesk-ui", "Media Player"),
        "notes": i18nd("gulldesk-ui", "Notes"),
        "photos": i18nd("gulldesk-ui", "Photos"),
        "taskmanager": i18nd("gulldesk-ui", "Task Manager"),
        "weather": i18nd("gulldesk-ui", "Weather")
    })
    readonly property var variants: ({
        "org.gulldesk.widget.clock#analog": i18nd("gulldesk-ui", "Analog clock"),
        "org.gulldesk.widget.clock#digital": i18nd("gulldesk-ui", "Digital clock"),
        "org.gulldesk.widget.clock#stacked": i18nd("gulldesk-ui", "Stacked clock"),
        "org.gulldesk.widget.weather#current": i18nd("gulldesk-ui", "Current weather"),
        "org.gulldesk.widget.weather#forecast": i18nd("gulldesk-ui", "Forecasted weather"),
        "org.gulldesk.widget.photo#cookie9": i18nd("gulldesk-ui", "Cookie"),
        "org.gulldesk.widget.photo#sunny": i18nd("gulldesk-ui", "Star"),
        "org.gulldesk.widget.photo#flower": i18nd("gulldesk-ui", "Flower"),
        "org.gulldesk.widget.photo#circle": i18nd("gulldesk-ui", "Circle"),
        "org.gulldesk.widget.photo#square": i18nd("gulldesk-ui", "Square"),
        "org.gulldesk.widget.photo#pill": i18nd("gulldesk-ui", "Pill"),
        "org.gulldesk.widget.photo#fill": i18nd("gulldesk-ui", "Full size")
    })
}
