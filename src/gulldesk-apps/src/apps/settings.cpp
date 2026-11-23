// SPDX-FileCopyrightText: 2026 GullDesk Developers
// SPDX-License-Identifier: GPL-2.0-or-later

#include "host.h"

static KLocalizedString displayName()
{
    return ki18nc("@title application name", "Settings");
}

int main(int argc, char **argv)
{
    static const GullDesk::Application application{"gulldesk-settings", "org.gulldesk.settings", "settings", "org.gulldesk.settings", true};
    return GullDesk::run(argc, argv, application, displayName);
}
