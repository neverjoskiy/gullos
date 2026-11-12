# GullOS System Architecture

This document provides a technical deep-dive into how **GullOS** boots, initializes hardware, enforces session locking on Wayland, executes its custom Rust terminal and shell, integrates Arch Linux's `pacman` binary package manager into a Buildroot `glibc` root filesystem, and persists state across reboots.

---

## 1. End-to-End Boot & Session Chain

```text
+-----------------------------------------------------------------------+
| Firmware (UEFI x86_64 or Legacy BIOS)                                 |
+-----------------------------------+-----------------------------------+
                                    |
                                    v
+-----------------------------------------------------------------------+
| Bootloader: GRUB2 (grubx64.efi + i386-pc hybrid MBR/GPT)              |
|  - Disk (.vdi / .img): Direct root=/dev/sda3 rw rootwait              |
|  - Live (.iso): Loads /boot/bzImage + /boot/initramfs-live.img        |
+-----------------------------------+-----------------------------------+
                                    |
                                    v
+-----------------------------------------------------------------------+
| Linux Kernel 6.12 LTS (x86_64)                                        |
|  - KMS/DRM: vmwgfx (VBox VMSVGA), vboxvideo, virtio_gpu, bochs,       |
|             simpledrm, i915, amdgpu                                   |
|  - Storage/FS: ahci, ata_piix, virtio_blk, ext4, squashfs, overlayfs  |
|  - IPC/Security: shmem, memfd_create, namespaces, seccomp, cgroups    |
+-----------------------------------+-----------------------------------+
                                    |
                                    v
+-----------------------------------------------------------------------+
| PID 1: BusyBox Init (/etc/inittab) + SysV RC (/etc/init.d/S*)         |
|  1. Mounts /proc, /sys, /dev (devtmpfs), /dev/pts, /dev/shm, /run     |
|  2. S10udev (eudev / systemd-udevd) populates DRM & libinput nodes    |
|  3. S15seatd launches seatd -g seat for rootless Wayland DRM access   |
|  4. S30dbus launches system D-Bus daemon                              |
|  5. S45network-dhcp starts dhcpcd across eth*/enp* interfaces         |
|  6. tty1 auto-launches /usr/bin/gull-session as user `gull` (uid 1000)|
|  7. tty2..tty4 spawn getty logins (demonstrating raw TTY fallback)    |
+-----------------------------------+-----------------------------------+
                                    |
                                    v
+-----------------------------------------------------------------------+
| Wayland Display Server: labwc (wlroots, pure Wayland, no Xwayland)    |
|  - Wrapped under `seatd-launch -- labwc -s /usr/libexec/gull-autostart`|
|  - Automatic software renderer fallback if VM lacks 3D GL acceleration|
|  - Styled with Catppuccin Mocha window borders & menus                |
+-----------------------------------+-----------------------------------+
                                    |
                                    v
+-----------------------------------------------------------------------+
| Session Gatekeeper: gull-lock (Rust)                                  |
|  - Invoked synchronously FIRST inside /usr/libexec/gull-autostart     |
|  - Binds ext-session-lock-v1 (or zwlr-layer-shell-v1 overlay fallback)|
|  - Blanks/locks all outputs with Catppuccin Mocha UI until unlocked   |
+-----------------------------------+-----------------------------------+
                                    |  (Authenticated)
                                    v
+-----------------------------------------------------------------------+
| Interactive Desktop: gull-term -> PTY -> gull-sh (Rust)               |
|  - Background painted to Catppuccin Mocha Base (#1e1e2e)              |
|  - gull-term opens Wayland wl_shm window & spawns gull-sh over PTY    |
|  - On raw tty2..tty4 (no WAYLAND_DISPLAY), gull-term execs gull-sh    |
+-----------------------------------------------------------------------+
```

---

## 2. Filesystem Layout & Merged `/usr` Strategy

A critical architectural challenge when combining **Buildroot** with **Arch Linux's `pacman`** is directory hierarchy compatibility:
- Traditional embedded Buildroot setups split `/bin`, `/sbin`, `/lib`, and `/usr/bin`, `/usr/lib`.
- Arch Linux packages (`filesystem`, `glibc`, `firefox`, `git`, `nano`) strictly assume **merged `/usr`**, where `/bin -> usr/bin`, `/sbin -> usr/bin`, `/lib -> usr/lib`, and `/lib64 -> usr/lib`.

GullOS resolves this cleanly by enabling `BR2_ROOTFS_MERGED_USR=y` in `buildroot-external/configs/gullos_defconfig` alongside `BR2_TOOLCHAIN_BUILDROOT_GLIBC=y`.

### Key Runtime Paths

| Path | Ownership & Purpose |
| :--- | :--- |
| `/boot/bzImage` | Custom Buildroot-compiled `x86_64` Linux kernel |
| `/boot/grub/grub.cfg` | GRUB2 configuration for UEFI + Legacy BIOS |
| `/usr/bin/gull-sh` | Custom Fish-style Rust shell (also registered in `/etc/shells`) |
| `/usr/bin/gull-term` | Custom Wayland terminal emulator with raw TTY fallback |
| `/usr/bin/gull-lock` | Custom Wayland `ext-session-lock-v1` screen locker |
| `/usr/bin/gull-session` | Sets `XDG_RUNTIME_DIR` (`/run/user/1000`), Wayland env vars (`MOZ_ENABLE_WAYLAND=1`), and starts `labwc` |
| `/usr/bin/gull-install` | Disk installer utility to clone live ISO system onto bare-metal disks |
| `/usr/libexec/gull-autostart` | Runs `gull-lock` synchronously before revealing desktop & launching `gull-term` |
| `/etc/pacman.conf` | Arch Linux `x86_64` binary repository definitions (`core`, `extra`) |
| `/etc/pacman.d/gnupg` | Pre-initialized & populated Arch Linux PGP keyring |
| `/var/lib/pacman/` | Synced repository databases (`sync/`) and installed package metadata (`local/`) |
| `/etc/gull-lock.conf` | SHA-256 salted/hashed lockscreen credential config |
| `/etc/sudoers.d/90-gull` | Grants `gull` (`wheel`) passwordless `sudo` (`0440` permissions) |
| `/home/gull/.config/labwc/` | Catppuccin Mocha `themerc`, `rc.xml`, `menu.xml`, and `environment` |
| `/home/gull/.gull_history` | Persistent command history file written by `gull-sh` |

---

## 3. Binary `pacman` Integration Inside Buildroot

Compiling heavyweight desktop browsers like `firefox` inside Buildroot from source takes hours, frequently exceeds GitHub Actions runner memory/time quotas, and lacks post-install package updates. GullOS instead decouples **base OS synthesis** from **binary package hydration**:

1. **Stage 1 — Base Synthesis (Buildroot)**:
   - Compiles the `x86_64` Linux kernel, `GRUB2` (BIOS `i386-pc` + UEFI `x86_64-efi`), `glibc` sysroot, `busybox`, `util-linux`, `eudev`, `dbus`, `dhcpcd`, `sudo`, `shadow`, and packages the custom Rust binaries (`gull-sh`, `gull-term`, `gull-lock`) via `buildroot-external/package/*`.
   - Emits an authoritative `rootfs.tar` archive preserving symbolic links and UID/GID tables.

2. **Stage 2 — Root-Privileged Binary Hydration (`scripts/pacman-bootstrap.sh`)**:
   - Why outside Buildroot's `fakeroot`? Running `gpg-agent` (`pacman-key --init`) inside `fakeroot` creates pseudo-owned Unix sockets and breaks `setuid` verification on `/usr/bin/sudo`. Running hydration in a real root staging directory (`rootfs-stage`) on the `ubuntu-latest` runner guarantees authentic UID `0` ownership, `04755` `setuid` bits on `/usr/bin/sudo`, and clean `gpgconf --kill all` teardown.
   - Downloads the official `archlinux-bootstrap-x86_64.tar.zst` onto the CI runner host (cached across runs).
   - Bind-mounts `/proc`, `/sys`, `/dev`, and `/etc/resolv.conf` and uses host `pacman --sysroot` / `chroot` to install pre-compiled `.pkg.tar.zst` packages directly from official Arch mirrors:
     - Core infrastructure: `pacman`, `archlinux-keyring`, `pacman-mirrorlist`, `ca-certificates`
     - Pure Wayland compositor stack: `labwc`, `seatd`, `wayland`, `mesa`, `libdrm`, `libinput`, `xkeyboard-config`, `pixman`
     - Pre-installed userland apps & fonts: `nano`, `git`, `firefox`, `ttf-dejavu`, `noto-fonts`
   - Executes `pacman-key --init` and `pacman-key --populate archlinux` inside the target rootfs so `sudo pacman -S <pkg>` works immediately on first boot without manual keyring commands.
   - Purges `/var/cache/pacman/pkg/*.pkg.tar.zst` while keeping `/var/lib/pacman/{sync,local}` intact, then re-applies GullOS's `/overlay` configs so Arch's `filesystem` package never overwrites GullOS's hostname (`gull-pc`), user `gull`, or init scripts.

---

## 4. Storage & Persistence Architecture

GullOS builds two distinct bootable artifacts from the hydrated rootfs:

### A. `gullos-x86_64.vdi` — Native Writable Disk Image (Recommended for VirtualBox)
- Constructed as a sparse `12 GiB` GPT disk image (`disk.raw`) partitioned into:
  1. `p1` (`1 MiB`, `ef02` BIOS Boot Partition for Legacy GRUB `core.img`)
  2. `p2` (`128 MiB`, `ef00` FAT32 EFI System Partition containing `/EFI/BOOT/BOOTX64.EFI`)
  3. `p3` (`~11.8 GiB`, `8300` `ext4` labeled `GULLOS_ROOT`, mounted `rw` at `/`)
- Converted to VirtualBox dynamic `.vdi` via `qemu-img convert -f raw -O vdi`.
- Every `sudo pacman -S ...`, system edit, and `/home/gull` file writes directly to `ext4` and persists natively across reboots.

### B. `gullos-x86_64.iso` — Hybrid Bootable Live/Installer ISO (BIOS + UEFI)
- Compresses `rootfs-stage` into `/live/rootfs.squashfs` (`zstd` compression).
- Generates `/boot/initramfs-live.img` containing a static/BusyBox `/init` script that:
  1. Mounts `/live/rootfs.squashfs` as the read-only `lowerdir`.
  2. Probes block devices for an optional partition labeled `GULLOS_PERSIST` (`ext4`). If found, mounts it as the `overlayfs` `upperdir` + `workdir` (giving USB sticks full reboot persistence!). Otherwise mounts a generous RAM `tmpfs` `upperdir`.
  3. Performs `switch_root` into the merged `/` filesystem.
- Includes `/usr/bin/gull-install`, allowing users who boot the `.iso` in a VM or on bare metal to partition, format, and install GullOS permanently onto an internal disk (e.g. `sudo gull-install /dev/sda`).

---

## 5. Rust Userland Subsystems

### `gull-lock` (`/rust-workspace/gull-lock`)
- Connects directly to Wayland via `wayland-client` (`0.31`).
- Primary path: binds `ext_session_lock_manager_v1` (`wayland-protocols` staging). Acquires the compositor-wide lock before rendering per-output `wl_shm` (`ARGB8888`/`XRGB8888`) surfaces, preventing any frame of desktop content from leaking prior to authentication.
- Fallback path: binds `zwlr_layer_shell_v1` on `Layer::Overlay` with `KeyboardInteractivity::Exclusive` anchored to all screen edges.
- Renders live time, date, `gull@gull-pc` identity pill, and interactive Catppuccin Mocha ring feedback via `ab_glyph` + pure-Rust fallback font rasterizer.

### `gull-term` (`/rust-workspace/gull-term`)
- **Startup Environment Probe**: Inspects `WAYLAND_DISPLAY` and verifies `wayland_client::Connection::connect_to_env()`. When absent or unreachable (such as on Linux virtual console `tty2`), immediately delegates execution (`CommandExt::exec`) to `/usr/bin/gull-sh` attached directly to the active TTY descriptors—satisfying the zero-crash raw console requirement.
- **Wayland Mode**: Uses `winit` (`wayland` + `wayland-dlopen`) and `softbuffer` (`wl_shm`) so VirtualBox VMs render cleanly regardless of 3D acceleration settings.
- Spawns `/usr/bin/gull-sh` inside a kernel pseudo-terminal (`portable-pty`), parses VT/ANSI sequences (`vte`), supports Catppuccin Mocha 16-color, 256-color, and 24-bit TrueColor (`CSI 38;2;r;g;bm`), scrollback history, and sends `TIOCSWINSZ` resize updates dynamically.

### `gull-sh` (`/rust-workspace/gull-sh`)
- Custom interactive shell built on `crossterm` raw terminal control:
  - **Lexer (`lexer.rs`)**: Splits pipelines, redirections, logical operators (`&&`, `||`, `;`), quoted strings, `$VAR` expansions, and `~` home paths.
  - **Highlighter (`highlighter.rs`)**: Re-colors the active buffer on every keystroke by checking command tokens against builtins and executable files cached from `$PATH`.
  - **Completer (`completer.rs`)**: Context-sensitive `Tab` completion (commands in command position; directories/files with `/` suffixes in argument position) plus Fish-style history prefix ghost text (`Overlay0` `#6c7086`, accepted via `Right` arrow or `Ctrl+F`).
  - **Executor (`executor.rs`)**: Spawns multi-stage OS pipelines (`Stdio::piped()`), file redirections, builtins (`cd`, `pwd`, `export`, `unset`, `alias`, `history`, `clear`, `help`, `exit`, `source`), `-c` one-shot execution, and script files.
