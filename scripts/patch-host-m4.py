#!/usr/bin/env python3
"""Patch Buildroot's host-m4 package for GCC 13+ compatibility.

Buildroot 2025.02 ships m4 1.4.19 whose bundled gnulib header lib/gl_oset.h
uses _GL_ATTRIBUTE_NODISCARD, which GCC 13+ on ubuntu-latest cannot parse.

This script inserts HOST_M4_POST_PATCH_HOOKS into package/host-m4/host-m4.mk
that strips _GL_ATTRIBUTE_NODISCARD from gl_oset.h after extraction but before
compilation.
"""

import sys
from pathlib import Path


def patch(host_m4_mk: Path) -> None:
    text = host_m4_mk.read_text()

    hook = """define HOST_M4_POST_PATCH_CMDS
\t$(SED) -i 's/_GL_ATTRIBUTE_NODISCARD//g' $(@D)/lib/gl_oset.h
endef
HOST_M4_POST_PATCH_HOOKS += HOST_M4_POST_PATCH_CMDS

"""

    # Insert hook before the $(eval $(host-...) line
    marker = "$(eval $(host-"
    lines = text.splitlines(True)
    inserted = False
    for i, line in enumerate(lines):
        if marker in line:
            lines.insert(i, hook)
            inserted = True
            break

    if not inserted:
        print(f"ERROR: could not find '{marker}' marker in {host_m4_mk}", file=sys.stderr)
        sys.exit(1)

    host_m4_mk.write_text("".join(lines))
    print(f"Patched {host_m4_mk} for GCC 13+ compatibility")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} <path-to-host-m4.mk>", file=sys.stderr)
        sys.exit(2)
    patch(Path(sys.argv[1]))
