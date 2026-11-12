# GullOS Top-Level Makefile
#
# IMPORTANT: Heavy OS builds (Buildroot, Kernel, Pacman chroot, ISO/VDI imaging)
# are engineered to run automatically on GitHub Actions (`ubuntu-latest`).
# Running `make` locally without `CI=true` or `GULLOS_ALLOW_LOCAL_BUILD=1` is blocked
# by default to protect developer workstations.

SHELL := /bin/bash

.PHONY: help ci all rust test fmt-check buildroot images clean

help:
	@echo "GullOS Build System (https://github.com/neverjoskiy/gullos)"
	@echo ""
	@echo "CI Targets (run on GitHub Actions ubuntu-latest):"
	@echo "  make ci          Build Rust userland, Buildroot+Kernel, Pacman rootfs, .iso & .vdi.zst"
	@echo "  make rust        Compile & test gull-sh, gull-term, and gull-lock (--release)"
	@echo "  make buildroot   Build Buildroot x86_64 glibc base system & Linux 6.12 kernel"
	@echo "  make images      Hydrate binary pacman packages and package .iso + .vdi.zst"
	@echo "  make fmt-check   Verify Rust workspace formatting"
	@echo "  make clean       Remove build/ and dist/ directories"

ci: all

all:
	@chmod +x ./scripts/build.sh
	@./scripts/build.sh all

rust:
	@chmod +x ./scripts/build.sh
	@./scripts/build.sh rust

buildroot:
	@chmod +x ./scripts/build.sh
	@./scripts/build.sh buildroot

images:
	@chmod +x ./scripts/build.sh
	@./scripts/build.sh images

test:
	@if [ "$${CI:-false}" != "true" ] && [ "$${GULLOS_ALLOW_LOCAL_BUILD:-0}" != "1" ]; then \
		echo "Blocked on local host: set GULLOS_ALLOW_LOCAL_BUILD=1 on Linux or push to GitHub Actions."; \
		exit 1; \
	fi
	cargo test --manifest-path rust-workspace/Cargo.toml --workspace

fmt-check:
	@if [ "$${CI:-false}" != "true" ] && [ "$${GULLOS_ALLOW_LOCAL_BUILD:-0}" != "1" ]; then \
		echo "Blocked on local host: set GULLOS_ALLOW_LOCAL_BUILD=1 on Linux or push to GitHub Actions."; \
		exit 1; \
	fi
	cargo fmt --manifest-path rust-workspace/Cargo.toml --all -- --check

clean:
	rm -rf build dist rust-workspace/target
