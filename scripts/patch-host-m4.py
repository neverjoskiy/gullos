#!/usr/bin/env python3
"""Patch Buildroot's host-m4 package for GCC 13+ compatibility.

Buildroot 2025.02 ships m4 1.4.19 whose bundled gnulib header lib/gl_oset.h
uses _GL_ATTRIBUTE_NODISCARD, which GCC 13+ on ubuntu-latest cannot parse,
causing:

    gl_oset.h:275:40: error: expected identifier or '(' before 'int'

This script inserts a HOST_M4_POST_EXTRACT_CMDS hook into
package/host-m4/host-m4.mk that strips _GL_ATTRIBUTE_NODISCARD from gl_oset.h
right after extraction, before compilation.
"""

import sys
from pathlib import Path


def patch(host_m4_mk: Path) -> None:
    text = host_m4_mk.read_text()

    hook = """define HOST_M4_POST_EXTRACT_CMDS
\t$(SED) -i "s/_GL_ATTRIBUTE_NODISCARD//g" $(@D)/lib/gl_oset.h
endef
HOST_M4_POST_EXTRACT_HOOKS += HOST_M4_POST_EXTRACT_CMDS

"""

    marker = "$(eval $(host-"
    for i, line in enumerate(text.splitlines(True)):
        if marker in line:
            lines = text.splitlines(True)
            lines.insert(i, hook)
            host_m4_mk.write_text("".join(lines))
            print(f"Patched {host_m4_mk} for GCC 13+ compatibility")
            return

    print(f"WARNING: could not find '{marker}' marker in {host_m4_mk}", file=sys.stderr)
    sys.exit(1)


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} <path-to-host-m4.mk>", file=sys.stderr)
        sys.exit(2)
    patch(Path(sys.argv[1]))
