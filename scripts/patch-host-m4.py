#!/usr/bin/env python3
"""Patch Buildroot's host-m4 package for GCC 13+ compatibility.

Buildroot ships m4 whose bundled gnulib headers use _GL_ATTRIBUTE_NODISCARD,
which GCC 13+ on the CI runner cannot parse (error: expected identifier or '('
before 'gl_list_node_t', etc.). This affects gl_oset.h, gl_list.h, gl_xlist.h
and any other gnulib header/source carrying the macro.

This script inserts HOST_M4_POST_PATCH_HOOKS into package/m4/m4.mk that
neutralises _GL_ATTRIBUTE_NODISCARD in every file under $(@D)/lib/ after
extraction but before compilation: the macro is redefined to expand to nothing
(the attribute itself is unusable in C mode), while every usage of the macro
name is removed so the declaration stays valid C.

Only files that actually contain the macro are rewritten, so timestamps of
unrelated sources (e.g. lib/Makefile.am) stay untouched and automake is not
triggered to regenerate Makefile.in.
"""

import sys
from pathlib import Path


def patch(host_m4_mk: Path) -> None:
    text = host_m4_mk.read_text()

    hook = """define HOST_M4_POST_PATCH_CMDS
\tgrep -rl '_GL_ATTRIBUTE_NODISCARD' $(@D)/lib 2>/dev/null | xargs -r perl -pi -e 'if (/^\\s*#\\s*define\\s+_GL_ATTRIBUTE_NODISCARD\\b/) { $$_ = "#define _GL_ATTRIBUTE_NODISCARD\\n"; } else { s/_GL_ATTRIBUTE_NODISCARD//g; }'
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
