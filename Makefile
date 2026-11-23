# GullOS Top-Level Makefile
#
# Heavy OS builds run on GitHub Actions (archlinux containers). Local runs blocked
# unless CI=true or GULLOS_ALLOW_LOCAL_BUILD=1.

SHELL := /bin/bash

.PHONY: help ci all rust test fmt-check packages iso vdi clean

help:
	@echo "GullOS Build System (https://github.com/neverjoskiy/gullos)"
	@echo ""
	@echo "CI Targets (run on GitHub Actions archlinux containers):"
	@echo "  make ci          Build packages + ISO + VDI"
	@echo "  make rust        Compile & test gull-sh, gull-term, gull-lock (--release)"
	@echo "  make packages    Build gull-* packages via makepkg into local repo"
	@echo "  make iso         Build the live/installer ISO (mkarchiso)"
	@echo "  make vdi         Build the persistent VirtualBox VDI (pacstrap)"
	@echo "  make fmt-check   Verify Rust workspace formatting"
	@echo "  make clean       Remove build/ and dist/ directories"

ci: all

all:
	@chmod +x ./scripts/build.sh
	@./scripts/build.sh all

rust:
	@chmod +x ./scripts/build.sh
	@cargo build --manifest-path rust-workspace/Cargo.toml --workspace --release

packages:
	@chmod +x ./scripts/build.sh
	@./scripts/build.sh packages

iso:
	@chmod +x ./scripts/build.sh
	@./scripts/build.sh iso

vdi:
	@chmod +x ./scripts/build.sh
	@./scripts/build.sh vdi

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
