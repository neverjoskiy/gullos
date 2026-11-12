# AI Context & Engineering Ruleset (`AI_CONTEXT.md`)

> **TARGET AUDIENCE:** Autonomous and interactive LLM coding agents (Claude, ZCode, Cursor, Windsurf, GitHub Copilot).
> **READ THIS ENTIRE FILE BEFORE MODIFYING ANY FILE IN THIS REPOSITORY.**

---

## 1. Non-Negotiable Operational Rules

1. **ZERO LOCAL HEAVY BUILDS OR DOWNLOADS**:
   - **NEVER** run `make`, `cargo build`, `wget`, `curl` (for tarballs/ISOs), `pacman`, or Buildroot commands on the developer's local Windows/macOS workstation unless the human explicitly overrides this rule.
   - All OS image compilation, Rust release compilation, Buildroot source fetching, and `pacman` rootfs hydration run **exclusively** inside GitHub Actions (`.github/workflows/build-os.yml` on `ubuntu-latest`).
2. **NO SOURCE COMPILATION FOR HEAVY ARCH APPS**:
   - **NEVER** enable `BR2_PACKAGE_FIREFOX`, `BR2_PACKAGE_GIT`, `BR2_PACKAGE_NANO`, Xorg/X11 (`BR2_PACKAGE_XORG7`), or audio daemons (`PipeWire`, `PulseAudio`, `ALSA` utils) in `buildroot-external/configs/gullos_defconfig`.
   - User-facing pre-built packages (`firefox`, `git`, `nano`, fonts, `labwc`, `seatd`) are installed strictly as pre-compiled Arch Linux `.pkg.tar.zst` binaries inside `scripts/pacman-bootstrap.sh`.
3. **UNIX `LF` LINE ENDINGS EVERYWHERE**:
   - All files under `overlay/`, `scripts/`, `buildroot-external/`, and `Makefile` execute inside Linux (`bash` / BusyBox `sh`). Never introduce `CRLF` (`\r\n`) line endings; respect `.gitattributes`.
4. **THEME INVARIANT — CATPPUCCIN MOCHA**:
   - All UI surfaces (`gull-lock`, `gull-term`, `gull-sh`, `labwc`) must strictly use the Catppuccin Mocha color tokens (`Base #1e1e2e`, `Mantle #181825`, `Crust #11111b`, `Surface0 #313244`, `Text #cdd6f4`, `Mauve #cba6f7`, `Green #a6e3a1`, `Red #f38ba8`, `Peach #fab387`, `Yellow #f9e2af`, `Sky #89dceb`, `Overlay0 #6c7086`).
5. **PURE WAYLAND + MANDATORY TTY FALLBACK**:
   - Never add X11 dependencies to Rust `Cargo.toml` manifests (`default-features = false` on `winit` and `softbuffer`, enabling only `wayland` + `wayland-dlopen`).
   - `gull-term` must **always** preserve its raw Linux TTY console fallback (`WAYLAND_DISPLAY` check -> `exec` `gull-sh`) so running `gull-term` on `/dev/tty2` never panics.

---

## 2. Authoritative Repository Map

| Directory / File | Responsibility |
| :--- | :--- |
| `.github/workflows/build-os.yml` | GitHub Actions workflow: disk space cleanup, Cargo & Buildroot `dl/` caching, `cargo test`, `scripts/build.sh`, Artifact upload, and GitHub Release publishing. |
| `Makefile` | Thin orchestrator wrapping `scripts/build.sh` with host-protection guardrails (`GULLOS_ALLOW_LOCAL_BUILD=1` required outside CI). |
| `scripts/build.sh` | Master CI build script: compiles `rust-workspace`, fetches Buildroot `2025.02`, applies `gullos_defconfig`, invokes `pacman-bootstrap.sh`, and builds `.vdi` + `.iso` + `SHA256SUMS`. |
| `scripts/pacman-bootstrap.sh` | Hydrates Buildroot's rootfs with Arch Linux binary `.pkg.tar.zst` packages, initializes `pacman-key` (`archlinux`), builds live `initramfs`, `.vdi`, and `.iso`. |
| `buildroot-external/external.desc` | Declares `BR2_EXTERNAL` name `GULLOS`. |
| `buildroot-external/external.mk` | Includes all `buildroot-external/package/*/*.mk` definitions. |
| `buildroot-external/Config.in` | Exposes `BR2_PACKAGE_GULL_SH`, `BR2_PACKAGE_GULL_TERM`, and `BR2_PACKAGE_GULL_LOCK`. |
| `buildroot-external/configs/gullos_defconfig` | Minimal `x86_64` `glibc` Buildroot configuration (`BR2_ROOTFS_MERGED_USR=y`, Linux `6.12`, BusyBox, `eudev`, `dbus`, `dhcpcd`, `sudo`, `GRUB2`). |
| `buildroot-external/board/gullos/linux.fragment` | Kernel Kconfig overrides for VirtualBox (`DRM_VMWGFX`, `DRM_VBOXVIDEO`, `VBOXGUEST`), VirtIO, KMS, `ext4`, `squashfs`, `overlayfs`, `memfd`. |
| `buildroot-external/board/gullos/users.txt` | Buildroot user table creating `gull` (`uid=1000`, groups `wheel,video,input,seat`). |
| `buildroot-external/board/gullos/post-build.sh` | Buildroot post-build hook normalizing permissions (`0440` sudoers, executable scripts) prior to `rootfs.tar` creation. |
| `buildroot-external/board/gullos/post-image.sh` | Buildroot post-image hook delegating final image assembly when invoked standalone. |
| `buildroot-external/board/gullos/grub-bios.cfg` | GRUB2 boot menu for persistent `.vdi` / raw disk boot (`root=LABEL=GULLOS_ROOT rw`). |
| `buildroot-external/board/gullos/grub-iso.cfg` | GRUB2 boot menu for live/installer `.iso` boot (`initramfs-live.img`). |
| `overlay/` | Static files copied verbatim onto target `/`: `/etc/pacman.conf`, `/etc/inittab`, `/etc/init.d/*`, `/usr/bin/gull-session`, `/usr/libexec/gull-autostart`, `/home/gull/.config/labwc/*`. |
| `rust-workspace/Cargo.toml` | Cargo workspace root uniting `gull-sh`, `gull-term`, and `gull-lock`. |
| `rust-workspace/gull-sh/` | Custom Fish-like interactive shell (`crossterm` raw editor, live syntax highlighting, `Tab` completion, `~/.gull_history`, pipeline/redirection executor). |
| `rust-workspace/gull-term/` | Custom Wayland terminal emulator (`winit` + `softbuffer` + `ab_glyph` + `vte` + `portable-pty`) + raw TTY console fallback. |
| `rust-workspace/gull-lock/` | Custom Wayland lockscreen (`wayland-client` + `ext-session-lock-v1` + `zwlr-layer-shell-v1` fallback + SHA-256 auth). |

---

## 3. Component Communication Contracts

1. **Init -> Session (`overlay/etc/inittab` -> `/usr/bin/gull-session`)**:
   - `tty1` spawns `su -l gull -c /usr/bin/gull-session` (or `login -f gull`).
   - `gull-session` creates `/run/user/1000` (`0700`), exports `XDG_SESSION_TYPE=wayland`, `MOZ_ENABLE_WAYLAND=1`, `WLR_NO_HARDWARE_CURSORS=1` (prevents invisible cursor bugs in VirtualBox VMSVGA), and starts `seatd-launch -- labwc -s /usr/libexec/gull-autostart`.
2. **Compositor -> Lockscreen -> Terminal (`/usr/libexec/gull-autostart`)**:
   - `gull-autostart` **blocks** on `/usr/bin/gull-lock` before launching `/usr/bin/gull-term`.
   - Never background `gull-lock` with `&` on boot without `ext-session-lock-v1` synchronization; blocking until `gull-lock` returns `0` ensures `gull-term` and desktop apps only launch after authentication succeeds.
3. **Terminal -> Shell (`gull-term` -> `gull-sh`)**:
   - `gull-term` resolves `$SHELL` or `/usr/bin/gull-sh` (falling back to `gull-sh` in `$PATH`, then `/bin/sh` only if `gull-sh` is missing).
   - `gull-term` sets `TERM=xterm-256color` and `COLORTERM=truecolor` on the child PTY so `gull-sh` and `pacman`/`nano`/`git` render full 24-bit Catppuccin Mocha colors.

---

## 4. Playbooks: How to Extend GullOS Safely

### Playbook A: Adding a New Pre-compiled Arch Binary Package
1. Open `scripts/pacman-bootstrap.sh`.
2. Append the Arch package name to the `PACMAN_PACKAGES` array (verify exact package naming in Arch `core`/`extra` repos).
3. Do **not** add it to `gullos_defconfig`.
4. Optionally add a launcher entry to `overlay/home/gull/.config/labwc/menu.xml` and keybinding in `overlay/home/gull/.config/labwc/rc.xml`.

### Playbook B: Adding a New Custom Rust Application (`gull-<app>`)
1. Create `rust-workspace/gull-<app>/Cargo.toml` and `src/main.rs`.
2. Register `"gull-<app>"` in `rust-workspace/Cargo.toml` `members`.
3. Create `buildroot-external/package/gull-<app>/Config.in` and `gull-<app>.mk` following the exact pattern in `buildroot-external/package/gull-term/` (installing the pre-built release binary from `$(GULLOS_RUST_TARGET_DIR)/release/gull-<app>` into `$(TARGET_DIR)/usr/bin/gull-<app>`).
4. Source the new `Config.in` in `buildroot-external/Config.in` and enable `BR2_PACKAGE_GULL_<APP>=y` in `buildroot-external/configs/gullos_defconfig`.

### Playbook C: Adding Kernel Drivers / Hardware Support
1. Add `CONFIG_<DRIVER>=y` (prefer built-in `=y` over `=m` for boot-critical storage, KMS, and network drivers so live/disk boots work even without module probing delays) in `buildroot-external/board/gullos/linux.fragment`.
2. Document any required VirtualBox settings in `docs/VIRTUALBOX.md`.
