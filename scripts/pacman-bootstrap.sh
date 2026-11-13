#!/usr/bin/env bash
# /scripts/pacman-bootstrap.sh
# Hydrates the Buildroot x86_64 glibc rootfs with pre-compiled Arch Linux binary
# packages (.pkg.tar.zst) via pacman, initializes the archlinux keyring inside
# chroot, and builds both:
#   1. dist/gullos-x86_64.vdi (.zst) - Persistent writable VirtualBox disk image
#   2. dist/gullos-x86_64.iso        - Hybrid BIOS/UEFI bootable live/installer ISO
#
# CRITICAL: Executed ONLY inside Linux CI (GitHub Actions ubuntu-24.04) as root.

set -euo pipefail

if [ "$(uname -s)" != "Linux" ]; then
    echo "ERROR: pacman-bootstrap.sh must only run on a Linux CI runner." >&2
    exit 1
fi

if [ "$(id -u)" -ne 0 ]; then
    echo "ERROR: pacman-bootstrap.sh must be invoked via sudo/root in CI." >&2
    exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOTFS_TAR="${1:?Usage: pacman-bootstrap.sh <rootfs.tar> <bzImage> <rust-release-dir> <work-dir> <dist-dir>}"
BZIMAGE="${2:?Missing bzImage path}"
RUST_RELEASE_DIR="${3:?Missing rust release dir}"
WORK_DIR="${4:?Missing work dir}"
DIST_DIR="${5:?Missing dist dir}"

CACHE_DIR="${WORK_DIR}/cache"
BOOT_EXTRACT="${WORK_DIR}/arch-host"
BOOT_ROOT="${BOOT_EXTRACT}/root.x86_64"
STAGE_DIR="${WORK_DIR}/rootfs-stage"
ISO_DIR="${WORK_DIR}/iso-root"
INITRAMFS_DIR="${WORK_DIR}/initramfs-live"
DISK_RAW="${WORK_DIR}/gullos-disk.raw"
LOOP_DEV=""

# Binary packages fetched directly from official Arch Linux x86_64 mirrors
# (NEVER compiled from source in Buildroot!)
PACMAN_PACKAGES=(
    filesystem
    glibc
    gcc-libs
    zlib
    zstd
    xz
    openssl
    ca-certificates
    curl
    pacman
    archlinux-keyring
    pacman-mirrorlist
    bash
    coreutils
    util-linux
    shadow
    sudo
    kbd
    e2fsprogs
    dosfstools
    parted
    grub
    dbus
    dhcpcd
    iproute2
    seatd
    wayland
    mesa
    libdrm
    libinput
    libxkbcommon
    xkeyboard-config
    pixman
    labwc
    swaybg
    nano
    git
    firefox
    ttf-dejavu
    noto-fonts
)

cleanup() {
    set +e
    # Terminate any ephemeral gpg-agent processes inside chroots before unmounting
    if [ -d "${STAGE_DIR}/etc/pacman.d/gnupg" ]; then
        chroot "${STAGE_DIR}" gpgconf --homedir /etc/pacman.d/gnupg --kill all >/dev/null 2>&1 || true
    fi
    if [ -d "${BOOT_ROOT}/etc/pacman.d/gnupg" ]; then
        chroot "${BOOT_ROOT}" gpgconf --homedir /etc/pacman.d/gnupg --kill all >/dev/null 2>&1 || true
    fi

    for mp in \
        "${STAGE_DIR}/dev/pts" "${STAGE_DIR}/dev" "${STAGE_DIR}/sys" "${STAGE_DIR}/proc" \
        "${BOOT_ROOT}/mnt/gullos" "${BOOT_ROOT}/dev/pts" "${BOOT_ROOT}/dev" "${BOOT_ROOT}/sys" "${BOOT_ROOT}/proc" \
        "${WORK_DIR}/mnt-efi" "${WORK_DIR}/mnt-root"; do
        if mountpoint -q "${mp}" 2>/dev/null; then
            umount -lf "${mp}" 2>/dev/null || true
        fi
    done

    if [ -n "${LOOP_DEV}" ] && [ -b "${LOOP_DEV}" ]; then
        losetup -d "${LOOP_DEV}" 2>/dev/null || true
    fi
}
trap cleanup EXIT

mkdir -p "${CACHE_DIR}" "${DIST_DIR}"
rm -rf "${STAGE_DIR}" "${ISO_DIR}" "${INITRAMFS_DIR}" "${DISK_RAW}"
mkdir -p "${STAGE_DIR}" "${ISO_DIR}/boot/grub" "${ISO_DIR}/live"

# ------------------------------------------------------------------------------
# 1. Fetch & Prepare Arch Linux Host Bootstrap Environment (for genuine pacman)
# ------------------------------------------------------------------------------
BOOTSTRAP_ARCHIVE="${CACHE_DIR}/archlinux-bootstrap-x86_64.tar.zst"
if [ ! -s "${BOOTSTRAP_ARCHIVE}" ]; then
    echo "[gullos:pacman] Downloading official Arch Linux x86_64 bootstrap tarball..."
    curl -fL --retry 4 --retry-delay 3 \
        -o "${BOOTSTRAP_ARCHIVE}.part" \
        "https://geo.mirror.pkgbuild.com/iso/latest/archlinux-bootstrap-x86_64.tar.zst" || \
    curl -fL --retry 4 --retry-delay 3 \
        -o "${BOOTSTRAP_ARCHIVE}.part" \
        "https://mirror.rackspace.com/archlinux/iso/latest/archlinux-bootstrap-x86_64.tar.zst"
    mv "${BOOTSTRAP_ARCHIVE}.part" "${BOOTSTRAP_ARCHIVE}"
fi

if [ ! -d "${BOOT_ROOT}/usr/bin" ]; then
    echo "[gullos:pacman] Extracting Arch bootstrap environment..."
    rm -rf "${BOOT_EXTRACT}"
    mkdir -p "${BOOT_EXTRACT}"
    tar --use-compress-program="zstd -d" -xpf "${BOOTSTRAP_ARCHIVE}" -C "${BOOT_EXTRACT}"
fi

cp -f "${REPO_ROOT}/overlay/etc/pacman.conf" "${BOOT_ROOT}/etc/pacman.conf"
cp -f "${REPO_ROOT}/overlay/etc/pacman.d/mirrorlist" "${BOOT_ROOT}/etc/pacman.d/mirrorlist"
cp -Lf /etc/resolv.conf "${BOOT_ROOT}/etc/resolv.conf"
mkdir -p "${BOOT_ROOT}/mnt/gullos" "${CACHE_DIR}/pacman-pkg"
mkdir -p "${BOOT_ROOT}/var/cache/pacman/pkg"

# ------------------------------------------------------------------------------
# 2. Extract Base Buildroot Rootfs & Preserve BusyBox Init Infrastructure
# ------------------------------------------------------------------------------
echo "[gullos:pacman] Extracting Buildroot base rootfs.tar..."
tar --numeric-owner -xpf "${ROOTFS_TAR}" -C "${STAGE_DIR}"

# Back up Buildroot's BusyBox binary & rcS/rcK scripts so BusyBox PID 1 remains intact
BACKUP_INIT="${WORK_DIR}/init-backup"
rm -rf "${BACKUP_INIT}"
mkdir -p "${BACKUP_INIT}/etc/init.d"
cp -a "${STAGE_DIR}/usr/bin/busybox" "${BACKUP_INIT}/busybox" 2>/dev/null || \
    cp -a "${STAGE_DIR}/bin/busybox" "${BACKUP_INIT}/busybox"
for rc_file in rcS rcK; do
    if [ -f "${STAGE_DIR}/etc/init.d/${rc_file}" ]; then
        cp -a "${STAGE_DIR}/etc/init.d/${rc_file}" "${BACKUP_INIT}/etc/init.d/${rc_file}"
    fi
done

mkdir -p \
    "${STAGE_DIR}/var/lib/pacman" \
    "${STAGE_DIR}/var/cache/pacman/pkg" \
    "${STAGE_DIR}/etc/pacman.d/gnupg" \
    "${STAGE_DIR}/proc" "${STAGE_DIR}/sys" "${STAGE_DIR}/dev/pts" "${STAGE_DIR}/run"

cp -f "${REPO_ROOT}/overlay/etc/pacman.conf" "${STAGE_DIR}/etc/pacman.conf"
cp -f "${REPO_ROOT}/overlay/etc/pacman.d/mirrorlist" "${STAGE_DIR}/etc/pacman.d/mirrorlist"

# ------------------------------------------------------------------------------
# 3. Install Pre-compiled Binary Packages (.pkg.tar.zst) via Pacman
# ------------------------------------------------------------------------------
echo "[gullos:pacman] Mounting chroot filesystems and initializing bootstrap keyring..."
mount -t proc proc "${BOOT_ROOT}/proc"
mount -t sysfs sys "${BOOT_ROOT}/sys"
mount --bind /dev "${BOOT_ROOT}/dev"
mount --bind /dev/pts "${BOOT_ROOT}/dev/pts"
mount --bind "${STAGE_DIR}" "${BOOT_ROOT}/mnt/gullos"

# Mount target virtual filesystems inside /mnt/gullos so pacman post-install hooks succeed
mount -t proc proc "${STAGE_DIR}/proc"
mount -t sysfs sys "${STAGE_DIR}/sys"
mount --bind /dev "${STAGE_DIR}/dev"
mount --bind /dev/pts "${STAGE_DIR}/dev/pts"

chroot "${BOOT_ROOT}" pacman-key --init
chroot "${BOOT_ROOT}" pacman-key --populate archlinux

echo "[gullos:pacman] Installing binary packages into GullOS rootfs..."
chroot "${BOOT_ROOT}" pacman \
    --root /mnt/gullos \
    --config /mnt/gullos/etc/pacman.conf \
    --cachedir /var/cache/pacman/pkg \
    --noconfirm \
    --needed \
    --overwrite '*' \
    -Sy "${PACMAN_PACKAGES[@]}"

chroot "${BOOT_ROOT}" gpgconf --homedir /etc/pacman.d/gnupg --kill all >/dev/null 2>&1 || true

umount -lf "${BOOT_ROOT}/mnt/gullos"
umount -lf "${BOOT_ROOT}/dev/pts"
umount -lf "${BOOT_ROOT}/dev"
umount -lf "${BOOT_ROOT}/sys"
umount -lf "${BOOT_ROOT}/proc"

# ------------------------------------------------------------------------------
# 4. Initialize Target Pacman Keyring & Re-apply GullOS Overlay / Credentials
# ------------------------------------------------------------------------------
echo "[gullos:pacman] Initializing pacman-key inside target GullOS rootfs..."
cp -Lf /etc/resolv.conf "${STAGE_DIR}/etc/resolv.conf"
chroot "${STAGE_DIR}" pacman-key --init
chroot "${STAGE_DIR}" pacman-key --populate archlinux
chroot "${STAGE_DIR}" gpgconf --homedir /etc/pacman.d/gnupg --kill all >/dev/null 2>&1 || true

# Clean any Residual gpg-agent sockets so mkfs.ext4 -d succeeds cleanly
find "${STAGE_DIR}/etc/pacman.d/gnupg" -type s -delete 2>/dev/null || true
rm -rf "${STAGE_DIR}/var/cache/pacman/pkg/"*.pkg.tar* 2>/dev/null || true

echo "[gullos:pacman] Restoring BusyBox init, GullOS overlay, Rust userland & user gull..."
install -m 0755 "${BACKUP_INIT}/busybox" "${STAGE_DIR}/usr/bin/busybox"
ln -sf /usr/bin/busybox "${STAGE_DIR}/usr/bin/init"
ln -sf ../bin/busybox "${STAGE_DIR}/usr/sbin/init" 2>/dev/null || true
ln -sf /usr/bin/busybox "${STAGE_DIR}/sbin/init"
mkdir -p "${STAGE_DIR}/etc/init.d"
for rc_file in rcS rcK; do
    if [ -f "${BACKUP_INIT}/etc/init.d/${rc_file}" ]; then
        install -m 0755 "${BACKUP_INIT}/etc/init.d/${rc_file}" "${STAGE_DIR}/etc/init.d/${rc_file}"
    fi
done
# Ensure standard rcS exists even if Buildroot skeleton differed
if [ ! -f "${STAGE_DIR}/etc/init.d/rcS" ]; then
    cat > "${STAGE_DIR}/etc/init.d/rcS" << 'EOF'
#!/bin/sh
for i in /etc/init.d/S??* ; do
    [ ! -f "$i" ] && continue
    "$i" start
done
EOF
    chmod 0755 "${STAGE_DIR}/etc/init.d/rcS"
fi
if [ ! -f "${STAGE_DIR}/etc/init.d/rcK" ]; then
    cat > "${STAGE_DIR}/etc/init.d/rcK" << 'EOF'
#!/bin/sh
for i in $(ls -r /etc/init.d/S??* 2>/dev/null) ; do
    [ ! -f "$i" ] && continue
    "$i" stop
done
EOF
    chmod 0755 "${STAGE_DIR}/etc/init.d/rcK"
fi

# Re-apply GullOS static overlay (overwriting Arch's generic os-release, hostname, pacman.conf)
cp -a "${REPO_ROOT}/overlay/." "${STAGE_DIR}/"

# Install GullOS custom Rust release binaries
install -D -m 0755 "${RUST_RELEASE_DIR}/gull-sh" "${STAGE_DIR}/usr/bin/gull-sh"
install -D -m 0755 "${RUST_RELEASE_DIR}/gull-term" "${STAGE_DIR}/usr/bin/gull-term"
install -D -m 0755 "${RUST_RELEASE_DIR}/gull-lock" "${STAGE_DIR}/usr/bin/gull-lock"

# Write verified SHA-256 hash of default password "gull" into /etc/gull-lock.conf
GULL_PASS_HASH="$(printf 'gull' | sha256sum | awk '{print $1}')"
cat > "${STAGE_DIR}/etc/gull-lock.conf" << EOF
# GullOS Session Lock Configuration
user=gull
hostname=gull-pc
theme=catppuccin-mocha
password_sha256=${GULL_PASS_HASH}
EOF
chmod 0644 "${STAGE_DIR}/etc/gull-lock.conf"

# Configure groups, user `gull`, password `gull`, font cache, and setuid sudo inside chroot
chroot "${STAGE_DIR}" /usr/bin/bash -e << 'CHROOT_SETUP'
grep -qxF "/usr/bin/gull-sh" /etc/shells || echo "/usr/bin/gull-sh" >> /etc/shells

for grp in wheel video render input tty seat; do
    getent group "$grp" >/dev/null || groupadd -r "$grp"
done

if ! id -u gull >/dev/null 2>&1; then
    useradd -m -u 1000 -s /usr/bin/gull-sh -G wheel,video,render,input,tty,seat gull
else
    usermod -s /usr/bin/gull-sh -aG wheel,video,render,input,tty,seat gull
fi

printf "root:gull\ngull:gull\n" | chpasswd

if command -v fc-cache >/dev/null 2>&1; then
    fc-cache -f >/dev/null 2>&1 || true
fi
CHROOT_SETUP

# Enforce ownership & setuid permissions
chown -R 1000:1000 "${STAGE_DIR}/home/gull"
chown 0:0 "${STAGE_DIR}/usr/bin/sudo" "${STAGE_DIR}/etc/sudoers" "${STAGE_DIR}/etc/sudoers.d/90-gull"
chmod 04755 "${STAGE_DIR}/usr/bin/sudo"
chmod 0440 "${STAGE_DIR}/etc/sudoers.d/90-gull"
chmod 0755 \
    "${STAGE_DIR}/usr/bin/gull-session" \
    "${STAGE_DIR}/usr/bin/gull-install" \
    "${STAGE_DIR}/usr/libexec/gull-autostart" \
    "${STAGE_DIR}/home/gull/.config/labwc/autostart" \
    "${STAGE_DIR}"/etc/init.d/S*

# Install kernel into /boot/bzImage
mkdir -p "${STAGE_DIR}/boot/grub"
cp -f "${BZIMAGE}" "${STAGE_DIR}/boot/bzImage"
cp -f "${REPO_ROOT}/buildroot-external/board/gullos/grub-bios.cfg" "${STAGE_DIR}/boot/grub/grub.cfg"

# Unmount target chroot virtual filesystems before imaging
umount -lf "${STAGE_DIR}/dev/pts"
umount -lf "${STAGE_DIR}/dev"
umount -lf "${STAGE_DIR}/sys"
umount -lf "${STAGE_DIR}/proc"
find "${STAGE_DIR}" -type s -delete 2>/dev/null || true

# ------------------------------------------------------------------------------
# 5. Build Persistent Disk Image -> VirtualBox .vdi (.zst)
# ------------------------------------------------------------------------------
echo "[gullos:image] Assembling 12 GiB persistent GPT disk image (BIOS + UEFI + ext4)..."
truncate -s 12G "${DISK_RAW}"
parted -s "${DISK_RAW}" mklabel gpt
parted -s "${DISK_RAW}" mkpart bios_grub 1MiB 3MiB
parted -s "${DISK_RAW}" set 1 bios_grub on
parted -s "${DISK_RAW}" mkpart ESP fat32 3MiB 131MiB
parted -s "${DISK_RAW}" set 2 esp on
parted -s "${DISK_RAW}" mkpart GULLOS_ROOT ext4 131MiB 100%

LOOP_DEV="$(losetup --find --show --partscan "${DISK_RAW}")"
udevadm settle || sleep 1

mkfs.vfat -F 32 -n GULLOS_EFI "${LOOP_DEV}p2"
mkfs.ext4 -F -L GULLOS_ROOT -d "${STAGE_DIR}" "${LOOP_DEV}p3"

mkdir -p "${WORK_DIR}/mnt-root" "${WORK_DIR}/mnt-efi"
mount "${LOOP_DEV}p3" "${WORK_DIR}/mnt-root"
mkdir -p "${WORK_DIR}/mnt-root/boot/efi"
mount "${LOOP_DEV}p2" "${WORK_DIR}/mnt-root/boot/efi"

# Install Legacy BIOS GRUB + Removable UEFI GRUB
grub-install --target=i386-pc --boot-directory="${WORK_DIR}/mnt-root/boot" "${LOOP_DEV}"
mkdir -p "${WORK_DIR}/mnt-root/boot/efi/EFI/BOOT"
grub-mkstandalone \
    -O x86_64-efi \
    -o "${WORK_DIR}/mnt-root/boot/efi/EFI/BOOT/BOOTX64.EFI" \
    "boot/grub/grub.cfg=${REPO_ROOT}/buildroot-external/board/gullos/grub-bios.cfg"

sync
umount "${WORK_DIR}/mnt-root/boot/efi"
umount "${WORK_DIR}/mnt-root"
losetup -d "${LOOP_DEV}"
LOOP_DEV=""

echo "[gullos:image] Converting raw GPT disk to VirtualBox dynamic VDI..."
rm -f "${DIST_DIR}/gullos-x86_64.vdi" "${DIST_DIR}/gullos-x86_64.vdi.zst"
qemu-img convert -f raw -O vdi "${DISK_RAW}" "${DIST_DIR}/gullos-x86_64.vdi"
zstd -T0 -10 --rm "${DIST_DIR}/gullos-x86_64.vdi" -o "${DIST_DIR}/gullos-x86_64.vdi.zst"
rm -f "${DISK_RAW}"

# ------------------------------------------------------------------------------
# 6. Build Bootable Hybrid Live / Installer ISO (gullos-x86_64.iso)
# ------------------------------------------------------------------------------
echo "[gullos:image] Compressing squashfs and building Live OverlayFS initramfs..."
mksquashfs "${STAGE_DIR}" "${ISO_DIR}/live/rootfs.squashfs" -comp zstd -Xcompression-level 12 -noappend

mkdir -p "${INITRAMFS_DIR}"/{bin,sbin,etc,proc,sys,dev,run,newroot}
cp -a "${BACKUP_INIT}/busybox" "${INITRAMFS_DIR}/bin/busybox"
for applet in sh mount umount mkdir sleep cat grep awk sed blkid switch_root mdev ls ln; do
    ln -sf busybox "${INITRAMFS_DIR}/bin/${applet}"
done

cat > "${INITRAMFS_DIR}/init" << 'LIVE_INIT'
#!/bin/sh
export PATH=/bin:/sbin

mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev 2>/dev/null || mdev -s
mkdir -p /run/live/medium /run/live/rofs /run/live/overlay

echo "[gullos-live] Searching for GullOS live media..."
FOUND=""
for attempt in 1 2 3 4 5 6 7 8 9 10; do
    mdev -s 2>/dev/null || true
    for dev in /dev/sr* /dev/sd* /dev/vd* /dev/nvme*; do
        [ -b "$dev" ] || continue
        if mount -o ro "$dev" /run/live/medium 2>/dev/null; then
            if [ -f /run/live/medium/live/rootfs.squashfs ]; then
                FOUND="$dev"
                break 2
            fi
            umount /run/live/medium 2>/dev/null || true
        fi
    done
    sleep 1
done

if [ -z "$FOUND" ]; then
    echo "[gullos-live] ERROR: Could not locate /live/rootfs.squashfs! Dropping to rescue shell."
    exec /bin/sh
fi

mount -t squashfs -o loop,ro /run/live/medium/live/rootfs.squashfs /run/live/rofs

# Use optional persistent partition (label GULLOS_PERSIST) if present, else tmpfs RAM overlay
PERSIST_DEV=""
for dev in /dev/sd* /dev/vd* /dev/nvme*; do
    [ -b "$dev" ] || continue
    if blkid "$dev" 2>/dev/null | grep -q 'LABEL="GULLOS_PERSIST"'; then
        PERSIST_DEV="$dev"
        break
    fi
done

if [ -n "$PERSIST_DEV" ] && mount "$PERSIST_DEV" /run/live/overlay 2>/dev/null; then
    echo "[gullos-live] Persistent storage attached from ${PERSIST_DEV}"
else
    mount -t tmpfs -o size=75%,mode=0755 tmpfs /run/live/overlay
    echo "[gullos-live] Running with tmpfs RAM overlay"
fi

mkdir -p /run/live/overlay/upper /run/live/overlay/work
mount -t overlay overlay \
    -o lowerdir=/run/live/rofs,upperdir=/run/live/overlay/upper,workdir=/run/live/overlay/work \
    /newroot

mkdir -p /newroot/run/live
mount --move /run/live /newroot/run/live
umount /sys /proc

exec switch_root /newroot /sbin/init
LIVE_INIT
chmod 0755 "${INITRAMFS_DIR}/init"

(
    cd "${INITRAMFS_DIR}"
    find . -print0 | cpio --null -ov --format=newc 2>/dev/null | zstd -T0 -10 > "${ISO_DIR}/boot/initramfs-live.img"
)

cp -f "${BZIMAGE}" "${ISO_DIR}/boot/bzImage"
cp -f "${REPO_ROOT}/buildroot-external/board/gullos/grub-iso.cfg" "${ISO_DIR}/boot/grub/grub.cfg"

echo "[gullos:image] Building hybrid BIOS+UEFI bootable ISO (gullos-x86_64.iso)..."
grub-mkrescue -o "${DIST_DIR}/gullos-x86_64.iso" "${ISO_DIR}" -- -volid "GULLOS_LIVE"

(
    cd "${DIST_DIR}"
    sha256sum gullos-x86_64.iso gullos-x86_64.vdi.zst > SHA256SUMS
)

echo "[gullos:image] Success! Artifacts ready in ${DIST_DIR}:"
ls -lh "${DIST_DIR}"
