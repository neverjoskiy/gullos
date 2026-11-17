#!/bin/bash
set -e

# Enable live services
systemctl --root=/ enable NetworkManager.service
systemctl --root=/ enable seatd.service
systemctl --root=/ enable systemd-timesyncd.service

# gull user already created by mkarchiso? Ensure it exists with correct groups.
if ! id gull >/dev/null 2>&1; then
    useradd -m -u 1000 -G wheel,video,input,seat -s /usr/bin/gull-sh gull
    echo 'gull:gull' | chpasswd
fi
chown -R gull:gull /home/gull

# Mark as live (no installed marker) — Calamares creates /etc/gullos/installed on target.
mkdir -p /etc/gullos
