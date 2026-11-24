#!/usr/bin/env bash
# /scripts/build.sh
# GullOS CI build orchestrator (Arch base): packages -> ISO -> VDI.
# Runs on ubuntu-latest inside privileged archlinux containers.
set -euo pipefail

STAGE="${1:-all}"

if [ "$(uname -s)" != "Linux" ]; then
    echo "ERROR: GullOS builds are restricted to Linux CI runners (detected: $(uname -s))." >&2
    exit 1
fi
if [ "${CI:-false}" != "true" ] && [ "${GULLOS_ALLOW_LOCAL_BUILD:-0}" != "1" ]; then
    echo "ERROR: Refusing heavy local OS build (CI is not true and GULLOS_ALLOW_LOCAL_BUILD!=1)." >&2
    exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_ROOT="${GULLOS_BUILD_ROOT:-${REPO_ROOT}/build}"
DIST_DIR="${REPO_ROOT}/dist"
LOCAL_REPO="${BUILD_ROOT}/gullos-local-repo"
PKG_DIR="${BUILD_ROOT}/pkgbuild-out"

mkdir -p "${BUILD_ROOT}" "${DIST_DIR}" "${LOCAL_REPO}" "${PKG_DIR}"

inject_cachyos_repo() {
    local conf="${REPO_ROOT}/profile/pacman.conf"
    local tmp="${conf}.tmp"
    awk '
        /# CACHYOS_REPO_BLOCK_START/ { print; inblk=1; next }
        /# CACHYOS_REPO_BLOCK_END/ {
            print "[cachyos]"
            print "SigLevel = Never"
            print "Server = https://mirror.cachyos.org/repo/x86_64/cachyos"
            print "# CACHYOS_REPO_BLOCK_END"
            inblk=0
            next
        }
        inblk { next }
        { print }
    ' "$conf" > "$tmp"
    mv "$tmp" "$conf"
}

build_packages() {
    echo "[gullos:build] Building gull-* packages with makepkg..."
    inject_cachyos_repo
    for d in gull-sh gull-term gull-lock calamares-settings-gullos gulldesk-desktop-shell gulldesk-desktop-themes gulldesk-desktop-icons gulldesk-desktop-cursors gulldesk-desktop-wallpapers gulldesk-desktop-widgets gulldesk-desktop-services gulldesk-apps gulldesk-wallpapers; do
    [ -d "${REPO_ROOT}/pkgbuilds/${d}" ] || continue
    if [ "$d" = "gulldesk-apps" ] && [ ! -d /usr/lib/cmake/KF6Screen ]; then
        echo "[gullos:build] SKIP $d (KF6Screen dev not available)"; continue
    fi
    echo "[gullos:build] makepkg: $d"
        ( cd "${REPO_ROOT}/pkgbuilds/${d}" && makepkg -sf --noconfirm )
        cp "${REPO_ROOT}/pkgbuilds/${d}"/*.pkg.tar.zst "${PKG_DIR}/" 2>/dev/null || true
    done
    rm -f "${LOCAL_REPO}"/*.pkg.tar.zst
    cp "${PKG_DIR}"/*.pkg.tar.zst "${LOCAL_REPO}/"
    ( cd "${LOCAL_REPO}" && repo-add gullos.db.tar.gz *.pkg.tar.zst )
    echo "[gullos:build] Local repo ready at ${LOCAL_REPO}"
}

build_iso() {
    echo "[gullos:build] Building ISO with mkarchiso..."
    inject_cachyos_repo
    sed -i "s#file:///gullos-local-repo#file://${LOCAL_REPO}#" "${REPO_ROOT}/profile/pacman.conf"
    # pacstrap wrapper to allow overwriting conflicting files (gulldesk overlay vs plasma)
    mkdir -p "${BUILD_ROOT}/bin"
    cat > "${BUILD_ROOT}/bin/pacstrap" <<'WRAPPER'
#!/bin/bash
exec /usr/bin/pacstrap --overwrite='*' "$@"
WRAPPER
    chmod +x "${BUILD_ROOT}/bin/pacstrap"
    export PATH="${BUILD_ROOT}/bin:${PATH}"
    mkarchiso -v -w "${BUILD_ROOT}/iso-work" -o "${DIST_DIR}" "${REPO_ROOT}/profile"
    # mkarchiso names the file <iso_name>-<iso_version>-<arch>.iso; normalize to the required artifact name
    for f in "${DIST_DIR}"/gullos*.iso; do
        [ -e "$f" ] || continue
        [ "$f" = "${DIST_DIR}/gullos-x86_64.iso" ] && continue
        mv -f "$f" "${DIST_DIR}/gullos-x86_64.iso"
    done
    test -s "${DIST_DIR}/gullos-x86_64.iso"
    ( cd "${DIST_DIR}" && sha256sum gullos-x86_64.iso > SHA256SUMS.iso )
    echo "[gullos:build] ISO written to ${DIST_DIR}/gullos-x86_64.iso"
}

build_vdi() {
    echo "[gullos:build] Building VDI via build-disk-image.sh..."
    inject_cachyos_repo
    sed -i "s#file:///gullos-local-repo#file://${LOCAL_REPO}#" "${REPO_ROOT}/profile/pacman.conf"
    bash "${REPO_ROOT}/scripts/build-disk-image.sh" "${BUILD_ROOT}/vdi-work" "${DIST_DIR}" "${LOCAL_REPO}"
    test -s "${DIST_DIR}/gullos-x86_64.vdi.zst"
    echo "[gullos:build] VDI written to ${DIST_DIR}/gullos-x86_64.vdi.zst"
}

case "${STAGE}" in
    packages) build_packages ;;
    iso)      build_iso ;;
    vdi)      build_vdi ;;
    all)
        build_packages
        build_iso
        build_vdi
        ;;
    *)
        echo "Usage: $0 [packages|iso|vdi|all]" >&2
        exit 1
        ;;
esac

echo "=================================================================="
echo "[gullos:build] Stage '${STAGE}' complete."
ls -lh "${DIST_DIR}" 2>/dev/null || true
echo "=================================================================="
