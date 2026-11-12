# GullOS

[![Build GullOS](https://github.com/neverjoskiy/gullos/actions/workflows/build-os.yml/badge.svg)](https://github.com/neverjoskiy/gullos/actions/workflows/build-os.yml)
![Architecture](https://img.shields.io/badge/arch-x86__64-89b4fa?style=flat-square)
![Display](https://img.shields.io/badge/display-Wayland%20(labwc)-cba6f7?style=flat-square)
![Userland](https://img.shields.io/badge/userland-Rust%20%2B%20Pacman-a6e3a1?style=flat-square)
![Theme](https://img.shields.io/badge/theme-Catppuccin%20Mocha-f5c2e7?style=flat-square)

**GullOS** is a minimalist, security-conscious `x86_64` Linux distribution engineered from scratch using **Buildroot (`glibc`)**, a pure **Wayland** graphical stack, **Arch Linux's `pacman`** binary package ecosystem, and a purpose-built **Rust** interactive userland (`gull-lock`, `gull-term`, and `gull-sh`) unified under the **Catppuccin Mocha** design language.

Repository: [`https://github.com/neverjoskiy/gullos`](https://github.com/neverjoskiy/gullos)

---

## Core Highlights

- **Minimal Buildroot + `glibc` Core**: Custom `x86_64` Linux `6.12` LTS kernel tuned for both **Oracle VirtualBox** (Windows 11 host) and bare-metal UEFI/BIOS hardware, using `BR2_ROOTFS_MERGED_USR=y` for binary compatibility with Arch Linux packages.
- **Pure Wayland Graphics (Zero X11, Zero Audio Bloat)**: Powered by `labwc` (`wlroots`) + `seatd` with software (`pixman`/`wl_shm`) and hardware (`vmwgfx`/`vboxvideo`/`virtio-gpu`/`i915`/`amdgpu`) rendering paths. No Xorg/Xwayland and no PipeWire/ALSA daemons.
- **Boot-First Rust Session Lock (`gull-lock`)**: Implements the Wayland `ext-session-lock-v1` protocol (with `wlr-layer-shell` exclusive overlay fallback) and engages immediately upon boot before exposing the desktop session.
- **Custom Rust Terminal & Shell (`gull-term` + `gull-sh`)**:
  - **`gull-term`**: Native Wayland terminal emulator (`winit` + `softbuffer` + `ab_glyph` + `vte` + `portable-pty`) with TrueColor ANSI rendering, scrollback, dynamic PTY resizing, and **automatic TTY console fallback** when launched outside a Wayland session.
  - **`gull-sh`**: Fish-inspired interactive shell featuring real-time command validation & syntax highlighting, `Tab` completion for `$PATH` binaries and paths, inline history autosuggestions, pipelines/redirections, and persistent `~/.gull_history`.
- **Out-of-the-Box Binary `pacman` Package Manager**: Pre-initialized `/etc/pacman.d/gnupg` keyring (`archlinux`) and pre-installed binary `.pkg.tar.zst` packages (`firefox`, `git`, `nano`, `ttf-dejavu`, `noto-fonts`) — never compiled from source.
- **True Reboot Persistence**: Produces both a **writable VirtualBox `.vdi` disk image** (where `sudo pacman -S <pkg>` and user files persist natively across reboots) and a **bootable hybrid `.iso`** (BIOS + UEFI live/install media with `overlayfs` support and `gull-install`).
- **100% Cloud-Compiled via GitHub Actions**: Zero local host compilation required. Every push to `main` or manual dispatch builds, tests, caches, and publishes bootable images on `ubuntu-latest`.

---

## Catppuccin Mocha Visual Identity

Every visual layer of GullOS — kernel VT palette, `gull-lock`, `labwc` window borders and root menu, `gull-term` ANSI colors, and `gull-sh` syntax tokens — uses the canonical **Catppuccin Mocha** palette:

| Role | Token | Hex | Usage in GullOS |
| :--- | :--- | :--- | :--- |
| Background | `Base` / `Crust` | `#1e1e2e` / `#11111b` | Desktop canvas, `gull-lock` backdrop, `gull-term` background |
| Elevated Surface | `Mantle` / `Surface0` | `#181825` / `#313244` | Lockscreen card, window titlebars, completion bar |
| Primary Text | `Text` | `#cdd6f4` | Terminal foreground, clock digits, standard output |
| Muted / Ghost | `Overlay0` | `#6c7086` | `gull-sh` inline history autosuggestions, subtle borders |
| Accent / Focus | `Mauve` / `Lavender` | `#cba6f7` / `#b4befe` | Active window border, prompt sigil `❯`, `$ENV` variables |
| Valid / Success | `Green` | `#a6e3a1` | Validated `$PATH`/builtin commands in `gull-sh`, unlock ring |
| Error / Invalid | `Red` | `#f38ba8` | Unknown commands in `gull-sh`, failed authentication state |
| Flags / Strings | `Peach` / `Yellow` | `#fab387` / `#f9e2af` | CLI flags (`--flag`) and quoted string literals |
| Operators | `Sky` | `#89dceb` | Pipes (`\|`), redirects (`>`, `<`), logical chains (`&&`) |

---

## Default Credentials & Session Shortcuts

| Setting | Value |
| :--- | :--- |
| **Hostname** | `gull-pc` |
| **Default User** | `gull` (`uid=1000`, `wheel`, `video`, `input`, `seat`) |
| **Lockscreen / Login Password** | `gull` |
| **Privilege Escalation** | Passwordless `sudo` (`sudo pacman -Syu`, etc.) |

### Wayland (`labwc`) Keybindings

| Shortcut | Action |
| :--- | :--- |
| `Super + Enter` (or `Alt + Enter`) | Launch `gull-term` (`gull-sh`) |
| `Super + B` | Launch Mozilla `firefox` (Wayland native `MOZ_ENABLE_WAYLAND=1`) |
| `Super + L` | Lock session with `gull-lock` |
| `Alt + F4` / `Super + Q` | Close focused window |
| `Alt + Tab` | Switch windows |
| `Desktop Right-Click` | Open Catppuccin Mocha `labwc` application menu |
| `Ctrl + Alt + F2` | Switch to raw Linux `tty2` (demonstrates `gull-term` TTY fallback) |

---

## Repository Layout

```text
gullos/
├── .github/workflows/build-os.yml   # Cloud OS builder, caching, artifact & release publisher
├── buildroot-external/              # BR2_EXTERNAL tree (defconfig, kernel, GRUB, packages)
├── overlay/                         # Target rootfs overlay (/etc, /usr, /home/gull, labwc)
├── rust-workspace/                  # Rust workspace: gull-sh, gull-term, gull-lock
├── scripts/                         # CI orchestrator (build.sh) & pacman binary bootstrapper
├── docs/VIRTUALBOX.md               # Windows 11 VirtualBox step-by-step setup guide
├── ARCHITECTURE.md                  # Deep technical breakdown of boot, rootfs & pacman
├── AI_CONTEXT.md                    # Strict architectural map & ruleset for AI coding agents
├── CONTRIBUTING.md                  # Contribution workflow & CI-first development rules
├── CODE_OF_CONDUCT.md               # Community standards
└── Makefile                         # Linux CI targets (protected against accidental local runs)
```

---

## Downloading & Running on Windows 11 (VirtualBox)

1. Open the [**GitHub Actions**](https://github.com/neverjoskiy/gullos/actions) tab on `https://github.com/neverjoskiy/gullos` (or visit [**Releases**](https://github.com/neverjoskiy/gullos/releases)).
2. Download the artifacts produced by `build-os.yml`:
   - **`gullos-x86_64.vdi.zst`** (Recommended for persistent daily use — extract `.vdi` using 7-Zip or `zstd -d` on Windows 11).
   - **`gullos-x86_64.iso`** (Live boot / bare-metal USB / installer image).
3. In **Oracle VirtualBox** on Windows 11:
   - **Name**: `GullOS` | **Type**: `Linux` | **Version**: `Arch Linux (64-bit)` or `Other Linux (64-bit)`
   - **Memory**: `4096 MB` RAM (minimum `2048 MB`; `4096 MB` recommended for Firefox)
   - **Processors**: `2–4 vCPUs` (enable PAE/NX)
   - **System -> Motherboard**: Enable **EFI** (works with both UEFI and Legacy BIOS)
   - **Display**: `128 MB` Video Memory, Graphics Controller: **`VMSVGA`** (3D Acceleration optional — GullOS works with or without it)
   - **Storage**: Attach **`gullos-x86_64.vdi`** to the **SATA (AHCI)** controller (or attach `gullos-x86_64.iso` to the Optical Drive)
   - **Network**: Adapter 1 -> **NAT**, Adapter Type: **`Intel PRO/1000 MT Desktop (82540EM)`** or **`Paravirtualized Network (virtio-net)`**
4. Boot the VM:
   - `gull-lock` appears immediately in Catppuccin Mocha. Type **`gull`** and press **Enter**.
   - `gull-term` opens automatically running `gull-sh`.
   - Test persistence: run `sudo pacman -Sy fastfetch && fastfetch`, reboot (`sudo reboot`), and confirm `fastfetch` remains installed!

For troubleshooting, bare-metal installation (`sudo gull-install /dev/sda`), and EFI/BIOS nuances, read [`docs/VIRTUALBOX.md`](./docs/VIRTUALBOX.md) and [`ARCHITECTURE.md`](./ARCHITECTURE.md).

---

## License

Released under the [MIT License](./LICENSE). Pre-compiled binary packages installed into rootfs images via `pacman` remain governed by their respective upstream licenses in Arch Linux.
