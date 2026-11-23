// SPDX-FileCopyrightText: 2026 GullDesk Developers
// SPDX-License-Identifier: GPL-2.0-or-later

#include "host.h"

#include <QNetworkAccessManager>
#include <QNetworkDiskCache>
#include <QQmlEngine>
#include <QQmlNetworkAccessManagerFactory>
#include <QStandardPaths>

namespace
{
class CachedNetwork : public QQmlNetworkAccessManagerFactory
{
public:
    QNetworkAccessManager *create(QObject *parent) override
    {
        auto *manager = new QNetworkAccessManager(parent);
        auto *cache = new QNetworkDiskCache(manager);
        cache->setCacheDirectory(QStandardPaths::writableLocation(QStandardPaths::CacheLocation) + QStringLiteral("/images"));
        cache->setMaximumCacheSize(256 * 1024 * 1024);
        manager->setCache(cache);
        return manager;
    }
};

void setup(QQmlEngine &engine)
{
    static CachedNetwork factory;
    engine.setNetworkAccessManagerFactory(&factory);
}
}

static KLocalizedString displayName()
{
    return ki18nc("@title application name", "Store");
}

int main(int argc, char **argv)
{
    static const GullDesk::Application application{"gulldesk-store", "org.gulldesk.store", "store", "org.gulldesk.store", true, setup};
    return GullDesk::run(argc, argv, application, displayName);
}
