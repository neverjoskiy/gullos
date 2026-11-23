// SPDX-FileCopyrightText: 2026 GullDesk Developers
// SPDX-License-Identifier: GPL-2.0-or-later

#pragma once

#include <QObject>
#include <QTimer>

class HomeIconSync : public QObject
{
public:
    HomeIconSync();

private:
    void sync();

    QTimer m_timer;
};
