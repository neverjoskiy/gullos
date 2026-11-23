// SPDX-FileCopyrightText: 2026 GullDesk Developers
// SPDX-License-Identifier: GPL-2.0-or-later

#pragma once

#include <QDateTime>
#include <QFileSystemWatcher>
#include <QObject>
#include <QTimer>

class AvatarSync : public QObject
{
public:
    AvatarSync();

private:
    void schedule();
    void sync();
    void publish(const QString &face);

    QString m_gulldeskrc;
    QFileSystemWatcher m_watcher;
    QTimer m_timer;
    QDateTime m_seen;
};
