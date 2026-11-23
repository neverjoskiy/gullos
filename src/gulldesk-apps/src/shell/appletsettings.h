// SPDX-FileCopyrightText: 2026 GullDesk Developers
// SPDX-License-Identifier: GPL-2.0-or-later

#pragma once

#include <QObject>
#include <QQuickItem>
#include <QtQml/qqmlregistration.h>

class AppletSettings : public QObject
{
    Q_OBJECT
    QML_ELEMENT

public:
    explicit AppletSettings(QObject *parent = nullptr);

    Q_INVOKABLE bool open(QObject *applet, QQuickItem *item);
};
