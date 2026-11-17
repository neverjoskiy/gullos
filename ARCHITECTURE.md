# GullOS System Architecture

Base: vanilla Arch Linux (archiso profile + pacstrap). Two kernels: linux-cachyos (default) + linux (fallback). Pure Wayland via labwc + seatd. Custom Rust userland: gull-sh, gull-term, gull-lock. Catppuccin Mocha throughout. Calamares installer. Persistent VDI via pacstrap.

Build: GitHub Actions in privileged archlinux containers. See .github/workflows/build-os.yml and scripts/build.sh.

Legacy Buildroot tree preserved on branch legacy/buildroot.
