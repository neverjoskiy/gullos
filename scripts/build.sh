#!/usr/bin/env bash
# /scripts/build.sh
# End-to-end GullOS CI build orchestrator (Rust Userland -> Buildroot -> Pacman Rootfs -> VDI + ISO)
#
# CRITICAL: Designed for Ubuntu x86_64 GitHub Actions runners (`ubuntu-latest`).
# Refuses execution on non-Linux hosts or local machines unless CI=true or GULLOS_ALLOW_LOCAL_BUILD=1.

set -euo pipefail

STAGE="${1:-all}"

if [ "$(uname -s)" != "Linux" ]; then
    echo "ERROR: GullOS builds are restricted to Linux CI runners (detected: $(uname -s))." >&2
    echo "Push to GitHub to trigger .github/workflows/build-os.yml on ubuntu-latest." >&2
    exit 1
fi

if [ "${CI:-false}" != "true" ] && [ "${GULLOS_ALLOW_LOCAL_BUILD:-0}" != "1" ]; then
    echo "ERROR: Refusing heavy local OS build (CI is not true and GULLOS_ALLOW_LOCAL_BUILD!=1)." >&2
    echo "To override intentionally on a dedicated Linux build box, set GULLOS_ALLOW_LOCAL_BUILD=1." >&2
    exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILDROOT_VERSION="${BUILDROOT_VERSION:-2025.02}"
BUILD_ROOT="${GULLOS_BUILD_ROOT:-${REPO_ROOT}/build}"
BR_SRC_DIR="${BUILD_ROOT}/buildroot-${BUILDROOT_VERSION}"
BR_OUT_DIR="${BUILD_ROOT}/buildroot-out"
BR_DL_DIR="${BR2_DL_DIR:-${BUILD_ROOT}/buildroot-dl}"
IMAGE_WORK_DIR="${BUILD_ROOT}/image-work"
DIST_DIR="${REPO_ROOT}/dist"
RUST_MANIFEST="${REPO_ROOT}/rust-workspace/Cargo.toml"
RUST_TARGET_DIR="${GULLOS_RUST_TARGET_DIR:-${REPO_ROOT}/rust-workspace/target}"

# Buildroot refuses $PATH entries containing spaces or empty segments; sanitize $PATH
CLEAN_PATH=""
IFS=':' read -ra PATH_PARTS <<< "${PATH}"
for part in "${PATH_PARTS[@]}"; do
    [ -z "${part}" ] && continue
    case "${part}" in
        *" "* | *"	"*) continue ;;
        *)
            if [ -z "${CLEAN_PATH}" ]; then
                CLEAN_PATH="${part}"
            else
                CLEAN_PATH="${CLEAN_PATH}:${part}"
            fi
            ;;
    esac
done
export PATH="${CLEAN_PATH}"

check_prereqs() {
    local missing=()
    local cmds=(
        cargo rustc make gcc g++ bc bison flex cpio rsync curl tar xz zstd
        mksquashfs xorriso grub-mkrescue grub-mkstandalone grub-install
        qemu-img parted mkfs.vfat mkfs.ext4 losetup sha256sum
    )
    for cmd in "${cmds[@]}"; do
        if ! command -v "${cmd}" >/dev/null 2>&1; then
            missing+=("${cmd}")
        fi
    done

    if [ "${#missing[@]}" -gt 0 ]; then
        echo "ERROR: Missing required host build utilities: ${missing[*]}" >&2
        exit 1
    fi
}

build_rust() {
    echo "=================================================================="
    echo "[gullos:build] Stage 1/3 — Compiling GullOS Rust Userland"
    echo "=================================================================="
    cargo test --manifest-path "${RUST_MANIFEST}" --target-dir "${RUST_TARGET_DIR}" --workspace
    cargo build --manifest-path "${RUST_MANIFEST}" --target-dir "${RUST_TARGET_DIR}" --workspace --release

    for bin in gull-sh gull-term gull-lock; do
        if [ ! -x "${RUST_TARGET_DIR}/release/${bin}" ]; then
            echo "ERROR: Expected Rust release binary missing: ${RUST_TARGET_DIR}/release/${bin}" >&2
            exit 1
        fi
    done
    ls -lh "${RUST_TARGET_DIR}/release/gull-sh" \
           "${RUST_TARGET_DIR}/release/gull-term" \
           "${RUST_TARGET_DIR}/release/gull-lock"
}

build_buildroot() {
    echo "=================================================================="
    echo "[gullos:build] Stage 2/3 — Building Buildroot ${BUILDROOT_VERSION} + Kernel 6.12"
    echo "=================================================================="
    mkdir -p "${BUILD_ROOT}" "${BR_DL_DIR}" "${BR_OUT_DIR}"

    if [ ! -f "${BR_SRC_DIR}/Makefile" ]; then
        local br_tar="${BR_DL_DIR}/buildroot-${BUILDROOT_VERSION}.tar.xz"
        if [ ! -s "${br_tar}" ]; then
            echo "[gullos:build] Downloading Buildroot ${BUILDROOT_VERSION}..."
            curl -fL --retry 4 --retry-delay 3 \
                -o "${br_tar}.part" \
                "https://buildroot.org/downloads/buildroot-${BUILDROOT_VERSION}.tar.xz" || \
            curl -fL --retry 4 --retry-delay 3 \
                -o "${br_tar}.part" \
                "https://github.com/buildroot/buildroot/archive/refs/tags/${BUILDROOT_VERSION}.tar.gz"
            mv "${br_tar}.part" "${br_tar}"
        fi
        rm -rf "${BR_SRC_DIR}"
        mkdir -p "${BR_SRC_DIR}"
        tar -xf "${br_tar}" --strip-components=1 -C "${BR_SRC_DIR}"
    fi

    # Ensure board scripts and overlay executables have Unix 0755 bits after git checkout
    chmod +x \
        "${REPO_ROOT}/buildroot-external/board/gullos/post-build.sh" \
        "${REPO_ROOT}/buildroot-external/board/gullos/post-image.sh" \
        "${REPO_ROOT}/scripts/pacman-bootstrap.sh" \
        "${REPO_ROOT}/overlay/usr/bin/gull-session" \
        "${REPO_ROOT}/overlay/usr/bin/gull-install" \
        "${REPO_ROOT}/overlay/usr/libexec/gull-autostart" \
        "${REPO_ROOT}/overlay/home/gull/.config/labwc/autostart" \
        "${REPO_ROOT}"/overlay/etc/init.d/S*

    # Patch host-m4 gnulib for GCC 13+ compatibility (_GL_ATTRIBUTE_NODISCARD
    # parse error in gl_oset.h). Buildroot 2025.02 ships m4 1.4.19 whose bundled
    # gnulib is incompatible with the system GCC on ubuntu-latest.
    HOST_M4_MK="${BR_SRC_DIR}/package/host-m4/host-m4.mk"
    if [ -f "${HOST_M4_MK}" ]; then
        python3 "${REPO_ROOT}/scripts/patch-host-m4.py" "${HOST_M4_MK}"
    fi

    make -C "${BR_SRC_DIR}" \
        O="${BR_OUT_DIR}" \
        BR2_EXTERNAL="${REPO_ROOT}/buildroot-external" \
        BR2_DL_DIR="${BR_DL_DIR}" \
        GULLOS_REPO_ROOT="${REPO_ROOT}" \
        GULLOS_RUST_TARGET_DIR="${RUST_TARGET_DIR}" \
        gullos_defconfig

    # FORCE_UNSAFE_CONFIGURE=1 allows Buildroot host-tar/host-m4 builds if CI runs as root
    FORCE_UNSAFE_CONFIGURE=1 make -C "${BR_OUT_DIR}" \
        BR2_DL_DIR="${BR_DL_DIR}" \
        GULLOS_REPO_ROOT="${REPO_ROOT}" \
        GULLOS_RUST_TARGET_DIR="${RUST_TARGET_DIR}" \
        -j"$(nproc)"
}

build_images() {
    echo "=================================================================="
    echo "[gullos:build] Stage 3/3 — Hydrating Binary Pacman Rootfs & Images"
    echo "=================================================================="
    local rootfs_tar="${BR_OUT_DIR}/images/rootfs.tar"
    local bzimage="${BR_OUT_DIR}/images/bzImage"

    if [ ! -f "${rootfs_tar}" ] || [ ! -f "${bzimage}" ]; then
        echo "ERROR: Buildroot artifacts not found in ${BR_OUT_DIR}/images/." >&2
        exit 1
    fi

    mkdir -p "${IMAGE_WORK_DIR}" "${DIST_DIR}"

    if [ "$(id -u)" -eq 0 ]; then
        "${REPO_ROOT}/scripts/pacman-bootstrap.sh" \
            "${rootfs_tar}" \
            "${bzimage}" \
            "${RUST_TARGET_DIR}/release" \
            "${IMAGE_WORK_DIR}" \
            "${DIST_DIR}"
    else
        sudo "${REPO_ROOT}/scripts/pacman-bootstrap.sh" \
            "${rootfs_tar}" \
            "${bzimage}" \
            "${RUST_TARGET_DIR}/release" \
            "${IMAGE_WORK_DIR}" \
            "${DIST_DIR}"
    fi

    test -s "${DIST_DIR}/gullos-x86_64.iso"
    test -s "${DIST_DIR}/gullos-x86_64.vdi.zst"
    test -s "${DIST_DIR}/SHA256SUMS"

    echo "=================================================================="
    echo "[gullos:build] GullOS Build Complete! Output Artifacts:"
    cat "${DIST_DIR}/SHA256SUMS"
    echo "=================================================================="
}

case "${STAGE}" in
    rust)
        build_rust
        ;;
    buildroot)
        check_prereqs
        build_buildroot
        ;;
    images)
        check_prereqs
        build_images
        ;;
    all)
        check_prereqs
        build_rust
        build_buildroot
        build_images
        ;;
    *)
        echo "Usage: $0 [all|rust|buildroot|images]" >&2
        exit 1
        ;;
esac
