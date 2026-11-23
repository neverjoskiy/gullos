// SPDX-FileCopyrightText: 2026 GullDesk Developers
// SPDX-License-Identifier: GPL-2.0-or-later

#include "host.h"

static KLocalizedString displayName()
{
    return ki18nc("@title application name", "Files");
}

int main(int argc, char **argv)
{
    static const GullDesk::Application application{"gulldesk-files", "org.gulldesk.files", "files", "org.gulldesk.files", false};
    return GullDesk::run(argc, argv, application, displayName);
}
