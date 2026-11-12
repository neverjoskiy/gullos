# GullOS BR2_EXTERNAL makefile entrypoint
# GULLOS_RUST_TARGET_DIR may be overridden by scripts/build.sh; defaults to repo rust-workspace/target
GULLOS_REPO_ROOT ?= $(abspath $(BR2_EXTERNAL_GULLOS_PATH)/..)
GULLOS_RUST_TARGET_DIR ?= $(GULLOS_REPO_ROOT)/rust-workspace/target

include $(sort $(wildcard $(BR2_EXTERNAL_GULLOS_PATH)/package/*/*.mk))
