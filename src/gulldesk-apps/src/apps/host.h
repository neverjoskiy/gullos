// SPDX-FileCopyrightText: 2026 GullDesk Developers
// SPDX-License-Identifier: GPL-2.0-or-later

#pragma once

#include <KLocalizedString>

class QQmlEngine;

namespace GullDesk
{
struct Application {
    const char *component;
    const char *desktopFileName;
    const char *folder;
    const char *icon;
    bool unique;
    void (*setup)(QQmlEngine &engine) = nullptr;
};

int run(int &argc, char **argv, const Application &application, KLocalizedString (*displayName)());
}
