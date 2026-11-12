# Contributing to GullOS

Thank you for your interest in contributing to **GullOS**! Whether you are refining the Rust userland (`gull-sh`, `gull-term`, `gull-lock`), tuning the Linux kernel config for new hardware, or improving Wayland styling, these guidelines keep development fast, reproducible, and safe.

---

## Cloud-First Build Philosophy

GullOS uses a **CI-offloaded build workflow**:
- **You do NOT need a high-spec Linux build server locally.** Windows 11, macOS, and lightweight laptops can edit Rust code, Buildroot configs, and overlays directly.
- Pushing a branch or opening a Pull Request triggers `.github/workflows/build-os.yml` on GitHub Actions (`ubuntu-latest`), which compiles the Rust workspace, builds the kernel & Buildroot rootfs, hydrates binary Arch packages via `pacman`, and produces downloadable `.vdi` and `.iso` artifacts.

---

## Development Guidelines

### 1. Rust Workspace (`/rust-workspace`)
- Target edition: **Rust 2021** (`stable` toolchain).
- Zero placeholders: Every PR must contain complete, warning-free, production-ready code (`cargo clippy --workspace -- -D warnings` on Linux).
- Pure Wayland: Never pull X11 client crates (`x11-dl`, `xcb`, `winit/x11`) into workspace dependencies.
- TTY Safety: Any change to `gull-term` must preserve graceful fallback to `gull-sh` when `WAYLAND_DISPLAY` is unset.

### 2. Buildroot & Pacman Rules (`/buildroot-external` & `/scripts`)
- Keep `gullos_defconfig` minimal. Only kernel, bootloader, init/core daemons, toolchain, and custom `gull-*` packages belong in Buildroot.
- Userland applications (`firefox`, `git`, `nano`, fonts, utilities) must be installed as pre-compiled Arch Linux `.pkg.tar.zst` packages in `scripts/pacman-bootstrap.sh`.
- Maintain **Unix LF** line endings on all shell scripts, `init.d` scripts, and configuration files (`.gitattributes` enforces this automatically).

### 3. Design & Theme Consistency
- Adhere strictly to the **Catppuccin Mocha** palette across terminal ANSI tables, lockscreen elements, shell syntax highlighting, and `labwc` configuration files.

---

## Pull Request Process

1. Fork [`https://github.com/neverjoskiy/gullos`](https://github.com/neverjoskiy/gullos) and create a feature branch from `main` (`git checkout -b feat/your-feature`).
2. Read [`AI_CONTEXT.md`](./AI_CONTEXT.md) and [`ARCHITECTURE.md`](./ARCHITECTURE.md) to verify architectural invariants.
3. Commit your changes using Conventional Commit prefixes (`feat:`, `fix:`, `docs:`, `ci:`, `refactor:`).
4. Open a Pull Request against `main` describing:
   - What changed and why.
   - How it was verified in VirtualBox (`.vdi` or `.iso` from the CI run).

By contributing to GullOS, you agree that your contributions will be licensed under the project's [MIT License](./LICENSE) and abide by our [Code of Conduct](./CODE_OF_CONDUCT.md).
