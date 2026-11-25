# GullOS

Vanilla Arch Linux base. CachyOS kernel by default + standard Arch kernel fallback. Pure Wayland. Catppuccin Mocha.

Migration from Buildroot preserved on branch legacy/buildroot.

See ARCHITECTURE.md, AI_CONTEXT.md, CONTRIBUTING.md.

## Desktop: GullDesk (Plasma 6)

GullOS now ships Plasma 6 (Wayland-only) with the GullDesk shell (renamed from
Kernora KED 2). Labwc/seatd have been removed. See `docs/GULLOS-DESKTOP.md`
and `docs/GULLDESK-DIVERTS.md` for details.

Packages: `gulldesk-desktop-shell`, `gulldesk-desktop-themes`,
`gulldesk-desktop-icons`, `gulldesk-desktop-cursors`,
`gulldesk-desktop-wallpapers`, `gulldesk-desktop-widgets`,
`gulldesk-desktop-services`, `gulldesk-apps`, `gulldesk-wallpapers`,
`calamares-settings-gullos`.
