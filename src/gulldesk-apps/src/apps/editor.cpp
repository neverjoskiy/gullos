// SPDX-FileCopyrightText: 2026 GullDesk Developers
// SPDX-License-Identifier: GPL-2.0-or-later

#include "host.h"

static KLocalizedString displayName()
{
    return ki18nc("@title application name", "Text Editor");
}

int main(int argc, char **argv)
{
    static const GullDesk::Application application{"gulldesk-editor", "org.gulldesk.editor", "editor", "org.gulldesk.editor", false};
    return GullDesk::run(argc, argv, application, displayName);
}
