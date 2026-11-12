################################################################################
#
# gull-term
#
################################################################################

GULL_TERM_VERSION = 0.1.0
GULL_TERM_SITE = $(GULLOS_REPO_ROOT)/rust-workspace
GULL_TERM_SITE_METHOD = local
GULL_TERM_LICENSE = MIT

define GULL_TERM_BUILD_CMDS
	if [ ! -x "$(GULLOS_RUST_TARGET_DIR)/release/gull-term" ]; then \
		cargo build --manifest-path "$(GULLOS_REPO_ROOT)/rust-workspace/Cargo.toml" \
			--target-dir "$(GULLOS_RUST_TARGET_DIR)" --release -p gull-term; \
	fi
endef

define GULL_TERM_INSTALL_TARGET_CMDS
	$(INSTALL) -D -m 0755 "$(GULLOS_RUST_TARGET_DIR)/release/gull-term" \
		"$(TARGET_DIR)/usr/bin/gull-term"
endef

$(eval $(generic-package))
