#!/usr/bin/env bash
# Buildroot BR2_ROOTFS_POST_IMAGE_SCRIPT
# Verifies Buildroot outputs; scripts/build.sh invokes scripts/pacman-bootstrap.sh
# under root privileges to hydrate Arch binary packages and assemble .vdi + .iso.
set -euo pipefail

IMAGES_DIR="$1"

echo "[gullos:post-image] Verifying Buildroot artifacts in ${IMAGES_DIR}..."
test -f "${IMAGES_DIR}/bzImage" || { echo "ERROR: Missing ${IMAGES_DIR}/bzImage" >&2; exit 1; }
test -f "${IMAGES_DIR}/rootfs.tar" || { echo "ERROR: Missing ${IMAGES_DIR}/rootfs.tar" >&2; exit 1; }
echo "[gullos:post-image] Kernel bzImage and base rootfs.tar verified."
