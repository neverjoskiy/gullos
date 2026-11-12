################################################################################
#
# gull-sh
#
################################################################################

GULL_SH_VERSION = 0.1.0
GULL_SH_SITE = $(GULLOS_REPO_ROOT)/rust-workspace
GULL_SH_SITE_METHOD = local
GULL_SH_LICENSE = MIT

define GULL_SH_BUILD_CMDS
	if [ ! -x "$(GULLOS_RUST_TARGET_DIR)/release/gull-sh" ]; then \
		cargo build --manifest-path "$(GULLOS_REPO_ROOT)/rust-workspace/Cargo.toml" \
			--target-dir "$(GULLOS_RUST_TARGET_DIR)" --release -p gull-sh; \
	fi
endef

define GULL_SH_INSTALL_TARGET_CMDS
	$(INSTALL) -D -m 0755 "$(GULLOS_RUST_TARGET_DIR)/release/gull-sh" \
		"$(TARGET_DIR)/usr/bin/gull-sh"
endef

$(eval $(generic-package))
