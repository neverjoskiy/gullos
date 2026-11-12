################################################################################
#
# gull-lock
#
################################################################################

GULL_LOCK_VERSION = 0.1.0
GULL_LOCK_SITE = $(GULLOS_REPO_ROOT)/rust-workspace
GULL_LOCK_SITE_METHOD = local
GULL_LOCK_LICENSE = MIT

define GULL_LOCK_BUILD_CMDS
	if [ ! -x "$(GULLOS_RUST_TARGET_DIR)/release/gull-lock" ]; then \
		cargo build --manifest-path "$(GULLOS_REPO_ROOT)/rust-workspace/Cargo.toml" \
			--target-dir "$(GULLOS_RUST_TARGET_DIR)" --release -p gull-lock; \
	fi
endef

define GULL_LOCK_INSTALL_TARGET_CMDS
	$(INSTALL) -D -m 0755 "$(GULLOS_RUST_TARGET_DIR)/release/gull-lock" \
		"$(TARGET_DIR)/usr/bin/gull-lock"
endef

$(eval $(generic-package))
