/*
    SPDX-FileCopyrightText: 2026 GullDesk Developers
    SPDX-License-Identifier: GPL-2.0-or-later
*/

.pragma library

var prefix = "org.gulldesk.widget.";

var apps = [
    { id: "battery", icon: "org.gulldesk.battery", plugins: ["org.gulldesk.widget.battery"] },
    { id: "calendar", icon: "org.gulldesk.calendar", plugins: ["org.gulldesk.widget.calendar"] },
    { id: "clock", icon: "org.gulldesk.clock", plugins: ["org.gulldesk.widget.clock#analog", "org.gulldesk.widget.clock#digital",
                                                       "org.gulldesk.widget.clock#stacked", "org.gulldesk.widget.stopwatch",
                                                       "org.gulldesk.widget.timer", "org.gulldesk.widget.worldclock"] },
    { id: "gulldesk", icon: "gulldesk-logo", plugins: ["org.gulldesk.widget.glance"] },
    { id: "player", icon: "org.gulldesk.player", plugins: ["org.gulldesk.widget.media"] },
    { id: "notes", icon: "org.gulldesk.notes", plugins: ["org.gulldesk.widget.note"] },
    { id: "photos", icon: "org.gulldesk.photos", plugins: ["org.gulldesk.widget.photo#cookie9", "org.gulldesk.widget.photo#sunny",
                                                         "org.gulldesk.widget.photo#flower", "org.gulldesk.widget.photo#circle",
                                                         "org.gulldesk.widget.photo#square", "org.gulldesk.widget.photo#pill",
                                                         "org.gulldesk.widget.photo#fill"] },
    { id: "taskmanager", icon: "org.gulldesk.taskmanager", plugins: ["org.gulldesk.widget.cputemperature"] },
    { id: "weather", icon: "org.gulldesk.weather", plugins: ["org.gulldesk.widget.weather#current",
                                                           "org.gulldesk.widget.weather#forecast"] }
];

var suggestions = [
    { id: "essentials", plugins: ["org.gulldesk.widget.clock#analog", "org.gulldesk.widget.weather#current", "org.gulldesk.widget.glance",
                                  "org.gulldesk.widget.calendar", "org.gulldesk.widget.battery"] },
    { id: "others", plugins: ["org.gulldesk.widget.media", "org.gulldesk.widget.note", "org.gulldesk.widget.photo"] }
];

var reconfigurable = ["org.gulldesk.widget.cputemperature", "org.gulldesk.widget.glance", "org.gulldesk.widget.photo",
                      "org.gulldesk.widget.weather", "org.gulldesk.widget.worldclock"];

var minimumSpans = {
    "org.gulldesk.widget.battery": [2, 2],
    "org.gulldesk.widget.calendar": [2, 3],
    "org.gulldesk.widget.cputemperature": [2, 3],
    "org.gulldesk.widget.glance": [2, 1],
    "org.gulldesk.widget.media": [2, 2],
    "org.gulldesk.widget.note": [2, 2],
    "org.gulldesk.widget.photo": [1, 2],
    "org.gulldesk.widget.stopwatch": [2, 2],
    "org.gulldesk.widget.timer": [1, 1],
    "org.gulldesk.widget.weather": [2, 2]
};

function minimumSpan(entry) {
    return minimumSpans[plugin(entry)] || [1, 1];
}

var spans = {
    "org.gulldesk.widget.battery": [2, 2],
    "org.gulldesk.widget.calendar": [3, 3],
    "org.gulldesk.widget.clock": [2, 3],
    "org.gulldesk.widget.clock#analog": [2, 3],
    "org.gulldesk.widget.clock#digital": [2, 2],
    "org.gulldesk.widget.clock#stacked": [2, 3],
    "org.gulldesk.widget.cputemperature": [2, 3],
    "org.gulldesk.widget.glance": [4, 1],
    "org.gulldesk.widget.media": [3, 2],
    "org.gulldesk.widget.note": [2, 2],
    "org.gulldesk.widget.photo": [2, 2],
    "org.gulldesk.widget.photo#cookie9": [2, 3],
    "org.gulldesk.widget.photo#sunny": [2, 3],
    "org.gulldesk.widget.photo#flower": [2, 3],
    "org.gulldesk.widget.photo#circle": [2, 3],
    "org.gulldesk.widget.photo#square": [2, 3],
    "org.gulldesk.widget.photo#pill": [2, 3],
    "org.gulldesk.widget.photo#fill": [3, 3],
    "org.gulldesk.widget.stopwatch": [2, 3],
    "org.gulldesk.widget.timer": [2, 3],
    "org.gulldesk.widget.weather": [2, 3],
    "org.gulldesk.widget.weather#current": [2, 3],
    "org.gulldesk.widget.weather#forecast": [3, 3],
    "org.gulldesk.widget.worldclock": [2, 2]
};

function plugin(entry) {
    return String(entry).split("#")[0];
}

function variant(entry) {
    var parts = String(entry).split("#");
    return parts.length > 1 ? parts[1] : "";
}

var variantKeys = {
    "org.gulldesk.widget.photo": "shape"
};

function config(entry) {
    var style = variant(entry);
    var result = {};
    if (style.length > 0) {
        result[variantKeys[plugin(entry)] || "style"] = style;
    }
    return result;
}

function span(entry) {
    return spans[entry] || spans[plugin(entry)] || [2, 2];
}

function kind(entry) {
    var name = plugin(entry);
    var base = name.indexOf(prefix) === 0 ? name.substring(prefix.length) : "";
    var style = variant(entry);
    return base.length > 0 && style.length > 0 ? base + "-" + style : base;
}

function variantKey(name) {
    return variantKeys[plugin(name)] || "style";
}

function appOf(name) {
    var target = plugin(name);
    for (var i = 0; i < apps.length; ++i) {
        for (var j = 0; j < apps[i].plugins.length; ++j) {
            if (plugin(apps[i].plugins[j]) === target) {
                return apps[i];
            }
        }
    }
    return null;
}

function entryOf(name, configuration) {
    var app = appOf(name);
    if (!app) {
        return "";
    }
    var target = plugin(name);
    var value = configuration ? configuration[variantKey(target)] : undefined;
    if (value !== undefined && value !== null && String(value).length > 0
            && app.plugins.indexOf(target + "#" + value) >= 0) {
        return target + "#" + value;
    }
    return app.plugins.indexOf(target) >= 0 ? target : "";
}
