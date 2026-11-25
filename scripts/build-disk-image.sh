#!/usr/bin/env bash
# scripts/build-disk-image.sh
# Builds a persistent VirtualBox VDI image of GullOS (installed system, not live squashfs).
# Runs only on Linux CI as root inside a privileged Arch container.
set -euo pipefail

if [ "$(uname -s)" != "Linux" ]; then
    echo "ERROR: build-disk-image.sh must run on Linux CI." >&2
    exit 1
fi
if [ "$(id -u)" -ne 0 ]; then
    echo "ERROR: build-disk-image.sh must run as root." >&2
    exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="${1:?usage: build-disk-image.sh <work-dir> <dist-dir> <local-repo-dir>}"
DIST_DIR="${2:?missing dist-dir}"
LOCAL_REPO="${3:?missing local-repo-dir}"

DISK_RAW="${WORK_DIR}/gullos-disk.raw"
DISK_SIZE="${GULLOS_DISK_SIZE:-16G}"
ESP_SIZE="${GULLOS_ESP_SIZE:-512M}"

mkdir -p "${WORK_DIR}" "${DIST_DIR}"

# 1. Create sparse raw disk
truncate -s "${DISK_SIZE}" "${DISK_RAW}"

# 2. Partition: GPT with ESP + root (ext4)
parted -s "${DISK_RAW}" mklabel gpt
parted -s "${DISK_RAW}" mkpart bios_grub 1MiB 2MiB
parted -s "${DISK_RAW}" set 1 bios_grub on
parted -s "${DISK_RAW}" mkpart ESP fat32 2MiB 514MiB
parted -s "${DISK_RAW}" set 2 esp on
parted -s "${DISK_RAW}" mkpart primary ext4 514MiB 100%

# 3. Attach loop device
# Use losetup with partition scanning; fall back to offset-based devices
LOOP_DEV=$(losetup -f --show -P "${DISK_RAW}")
trap 'losetup -d "${LOOP_DEV}" 2>/dev/null || true' EXIT
partprobe "${LOOP_DEV}" 2>/dev/null || true
sleep 2

if [ -b "${LOOP_DEV}p2" ] && [ -b "${LOOP_DEV}p3" ]; then
    ESP_DEV="${LOOP_DEV}p2"
    ROOT_DEV="${LOOP_DEV}p3"
else
    # Fallback: offset-based loop devices (ESP starts at 1MiB, root at 513MiB)
    ESP_OFFSET=$((2 * 1024 * 1024))
    ROOT_OFFSET=$((514 * 1024 * 1024))
    ESP_DEV=$(losetup -f --show -o ${ESP_OFFSET} "${DISK_RAW}")
    ROOT_DEV=$(losetup -f --show -o ${ROOT_OFFSET} "${DISK_RAW}")
    trap 'losetup -d "${ESP_DEV}" 2>/dev/null; losetup -d "${ROOT_DEV}" 2>/dev/null; losetup -d "${LOOP_DEV}" 2>/dev/null || true' EXIT
fi

mkfs.vfat -F32 -n GULLOS_ESP "${ESP_DEV}"
mkfs.ext4 -L GULLOS_ROOT -F "${ROOT_DEV}"

ESP_MNT="${WORK_DIR}/esp-mnt"
ROOT_MNT="${WORK_DIR}/root-mnt"
mkdir -p "${ESP_MNT}" "${ROOT_MNT}"
# Wrap pacman to allow overwriting conflicting files (gulldesk overlay vs plasma)
mkdir -p "${WORK_DIR}/bin"
cat > "${WORK_DIR}/bin/pacman" <<'WRAPPER'
#!/bin/bash
exec /usr/bin/pacman --overwrite='*' "$@"
WRAPPER
chmod +x "${WORK_DIR}/bin/pacman"
export PATH="${WORK_DIR}/bin:${PATH}"
mount "${ROOT_DEV}" "${ROOT_MNT}"
mkdir -p "${ROOT_MNT}/boot/efi"
mount "${ESP_DEV}" "${ROOT_MNT}/boot/efi"

# 4. pacstrap the installed-system package set
PACSTRAP_PKGS=(
    base base-devel linux-cachyos linux linux-firmware
    networkmanager pipewire pipewire-pulse wireplumber rtkit
    firefox git nano ttf-dejavu ttf-roboto noto-fonts grim slurp wl-clipboard xdg-desktop-portal-wlr
    sudo man-db pacman-contrib grub efibootmgr os-prober
    gull-sh gull-term gull-lock calamares-settings-gullos
    plasma-desktop plasma-workspace kwin kglobalacceld kded kscreen powerdevil bluedevil plasma-nm plasma-pa
    kvantum breeze-gtk
    mkinitcpio
)

# Initialize pacman keyring (required for package signature verification)
pacman-key --init 2>/dev/null || true
pacman-key --populate archlinux 2>/dev/null || true
pacman-key --populate cachyos 2>/dev/null || true

# Make local gullos repo available to pacstrap via a bind mount + pacman.conf tweak
mkdir -p "${ROOT_MNT}/var/lib/pacman" 2>/dev/null || true
cp /etc/pacman.conf "${WORK_DIR}/pacman-disk.conf" 2>/dev/null || true

pacstrap -C "${REPO_ROOT}/profile/pacman.conf" "${ROOT_MNT}" "${PACSTRAP_PKGS[@]}" \
    --cachedir /var/cache/pacman/pkg

# 5. Configure the installed system
arch-chroot "${ROOT_MNT}" bash -s <<'CHROOT'
set -e
ln -sf /usr/share/zoneinfo/UTC /etc/localtime
echo "en_US.UTF-8 UTF-8" > /etc/locale.gen
locale-gen
echo "LANG=en_US.UTF-8" > /etc/locale.conf
echo "gull-pc" > /etc/hostname
cat > /etc/hosts <<'EOF'
127.0.0.1 localhost
127.0.1.1 gull-pc.localdomain gull-pc
::1 localhost
EOF
mkdir -p /etc/gullos
touch /etc/gullos/installed

# User gull (demo identity): password gull, passwordless sudo, autologin -> gull-lock
useradd -m -u 1000 -G wheel,video,input -s /usr/bin/gull-sh gull || true
GULL_HASH=$(openssl passwd -6 gull)
usermod -p "$GULL_HASH" gull 2>/dev/null || true
echo 'gull ALL=(ALL:ALL) NOPASSWD: ALL' > /etc/sudoers.d/90-gull
chmod 0440 /etc/sudoers.d/90-gull
chown -R gull:gull /home/gull

# getty autologin on tty1
mkdir -p /etc/systemd/system/getty@tty1.service.d
cat > /etc/systemd/system/getty@tty1.service.d/autologin.conf <<'EOF'
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin gull --noclear %I $TERM
EOF

# Enable services
systemctl enable NetworkManager.service
systemctl enable systemd-timesyncd.service
systemctl enable sddm.service 2>/dev/null || true

# GRUB for both kernels, CachyOS default, timeout 5s
cat > /etc/default/grub <<'EOF'
GRUB_DEFAULT=0
GRUB_TIMEOUT=5
GRUB_DISTRIBUTOR="GullOS"
GRUB_CMDLINE_LINUX_DEFAULT="quiet"
GRUB_CMDLINE_LINUX=""
EOF
# register both kernels
mkdir -p /boot/grub
grub-mkconfig -o /boot/grub/grub.cfg

# initramfs for both kernels
mkinitcpio -p linux 2>/dev/null || true
mkinitcpio -p linux-cachyos 2>/dev/null || true
CHROOT

# 6. Install GRUB to ESP + disk
arch-chroot "${ROOT_MNT}" grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=GullOS --removable
arch-chroot "${ROOT_MNT}" grub-install --target=i386-pc "${LOOP_DEV}" || true

# 7. Unmount
sync
umount "${ROOT_MNT}/boot/efi" 2>/dev/null || true
umount "${ROOT_MNT}" 2>/dev/null || true
losetup -d "${LOOP_DEV}" 2>/dev/null || true
trap - EXIT

# 8. Convert to VDI and compress
qemu-img convert -f raw -O vdi "${DISK_RAW}" "${WORK_DIR}/gullos-x86_64.vdi"
zstd -19 --rm -o "${DIST_DIR}/gullos-x86_64.vdi.zst" "${WORK_DIR}/gullos-x86_64.vdi" 2>/dev/null || \
    zstd -19 -o "${DIST_DIR}/gullos-x86_64.vdi.zst" "${WORK_DIR}/gullos-x86_64.vdi"
sha256sum "${DIST_DIR}/gullos-x86_64.vdi.zst" > "${DIST_DIR}/SHA256SUMS.vdi" 2>/dev/null || true

echo "[build-disk-image] VDI written to ${DIST_DIR}/gullos-x86_64.vdi.zst"
