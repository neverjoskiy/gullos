// SPDX-FileCopyrightText: 2026 GullDesk Developers
// SPDX-License-Identifier: GPL-2.0-or-later

#include "host.h"

static KLocalizedString displayName()
{
    return ki18nc("@title application name", "Photos");
}

int main(int argc, char **argv)
{
    static const GullDesk::Application application{"gulldesk-photos", "org.gulldesk.photos", "photos", "org.gulldesk.photos", false};
    return GullDesk::run(argc, argv, application, displayName);
}
