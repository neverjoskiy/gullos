// SPDX-FileCopyrightText: 2026 GullDesk Developers
// SPDX-License-Identifier: GPL-2.0-or-later

let watched = null;

workspace.windowAdded.connect(window => {
    if (!watched && window && window.appletPopup === true) {
        watched = window;
    }
});

workspace.windowRemoved.connect(window => {
    if (watched && window === watched) {
        watched = null;
        callDBus("invalid.gulldesk.gulldesk-welcome", "/invalid/gulldesk/gulldesk_welcome",
                 "org.freedesktop.Application", "Activate", {});
    }
});
