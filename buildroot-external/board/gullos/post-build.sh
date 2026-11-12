#!/usr/bin/env bash
# Buildroot BR2_ROOTFS_POST_BUILD_SCRIPT
# Normalizes permissions, shells, and init links inside $TARGET_DIR prior to rootfs.tar creation.
set -euo pipefail

TARGET_DIR="$1"

echo "[gullos:post-build] Finalizing Buildroot overlay permissions in ${TARGET_DIR}..."

# Ensure merged /usr directories exist
mkdir -p \
    "${TARGET_DIR}/boot/grub" \
    "${TARGET_DIR}/etc/pacman.d" \
    "${TARGET_DIR}/etc/sudoers.d" \
    "${TARGET_DIR}/run" \
    "${TARGET_DIR}/var/lib/pacman" \
    "${TARGET_DIR}/var/cache/pacman/pkg" \
    "${TARGET_DIR}/home/gull/.config/labwc"

# Register gull-sh in /etc/shells
if [ -f "${TARGET_DIR}/etc/shells" ]; then
    grep -qxF "/usr/bin/gull-sh" "${TARGET_DIR}/etc/shells" || echo "/usr/bin/gull-sh" >> "${TARGET_DIR}/etc/shells"
else
    printf "/bin/sh\n/bin/bash\n/usr/bin/gull-sh\n" > "${TARGET_DIR}/etc/shells"
fi

# Enforce strict permissions on sudoers and executable scripts
if [ -f "${TARGET_DIR}/etc/sudoers.d/90-gull" ]; then
    chmod 0440 "${TARGET_DIR}/etc/sudoers.d/90-gull"
fi

chmod 0755 "${TARGET_DIR}/usr/bin/gull-session" \
           "${TARGET_DIR}/usr/bin/gull-install" \
           "${TARGET_DIR}/usr/libexec/gull-autostart" \
           "${TARGET_DIR}/home/gull/.config/labwc/autostart"

for init_script in "${TARGET_DIR}"/etc/init.d/S*; do
    [ -e "${init_script}" ] && chmod 0755 "${init_script}"
done

# Copy bzImage into /boot inside rootfs if already built
if [ -f "${BINARIES_DIR:-}/bzImage" ]; then
    cp -f "${BINARIES_DIR}/bzImage" "${TARGET_DIR}/boot/bzImage"
fi

echo "[gullos:post-build] Base rootfs target preparation complete."
