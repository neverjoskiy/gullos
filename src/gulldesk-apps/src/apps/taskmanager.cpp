// SPDX-FileCopyrightText: 2026 GullDesk Developers
// SPDX-License-Identifier: GPL-2.0-or-later

#include "host.h"

static KLocalizedString displayName()
{
    return ki18nc("@title application name", "Task Manager");
}

int main(int argc, char **argv)
{
    static const GullDesk::Application application{"gulldesk-taskmanager", "org.gulldesk.taskmanager", "taskmanager", "org.gulldesk.taskmanager", true};
    return GullDesk::run(argc, argv, application, displayName);
}
