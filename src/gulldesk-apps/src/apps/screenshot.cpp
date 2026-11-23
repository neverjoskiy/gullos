// SPDX-FileCopyrightText: 2026 GullDesk Developers
// SPDX-License-Identifier: GPL-2.0-or-later

#include "host.h"

static KLocalizedString displayName()
{
    return ki18nc("@title application name", "Screenshot");
}

int main(int argc, char **argv)
{
    static const GullDesk::Application application{"gulldesk-screenshot", "org.gulldesk.screenshot", "screenshot", "org.gulldesk.screenshot", true};
    return GullDesk::run(argc, argv, application, displayName);
}
