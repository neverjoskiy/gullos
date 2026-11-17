# AI Context & Engineering Ruleset

Base is vanilla Arch Linux. No Buildroot. Packaging via PKGBUILDs in pkgbuilds/. ISO via mkarchiso (profile/). VDI via pacstrap (scripts/build-disk-image.sh). Kernels: linux-cachyos (default) + linux. Wayland-only (labwc+seatd). Catppuccin Mocha tokens. Rust workspace unchanged (rust-workspace/). gull-lock boot-first on installed systems (marker /etc/gullos/installed). Live has no lock. Calamares installer launched via gull-install.

Cloud-only builds: no local cargo/make/archiso. LF endings. No X11.
