// SPDX-FileCopyrightText: 2026 GullDesk Developers
// SPDX-License-Identifier: GPL-2.0-or-later

#include "host.h"

#include <QGuiApplication>
#include <QRegularExpression>

static KLocalizedString displayName()
{
    return ki18nc("@title application name", "Android apps");
}

static void useAppIdentity(QQmlEngine &)
{
    static const QRegularExpression package(QStringLiteral("^[A-Za-z0-9_]+(\\.[A-Za-z0-9_]+)+$"));
    const QStringList arguments = QCoreApplication::arguments().mid(1);
    for (const QString &argument : arguments) {
        if (package.match(argument).hasMatch()) {
            QGuiApplication::setDesktopFileName(QStringLiteral("waydroid.") + argument);
            return;
        }
    }
}

int main(int argc, char **argv)
{
    static const GullDesk::Application application{"gulldesk-appsplash", "org.gulldesk.appsplash", "appsplash", "org.gulldesk.android", false,
                                                  useAppIdentity};
    return GullDesk::run(argc, argv, application, displayName);
}
