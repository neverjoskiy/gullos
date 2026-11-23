#!/bin/sh
file="${XDG_CONFIG_HOME:-$HOME/.config}/kscreenlockerrc"
if [ -f "$file" ] && grep -q 'org\.gulldesk\.tower' "$file"; then
  sed -i 's/^\([Ww]allpaper[Pp]lugin=\)org\.gulldesk\.tower$/\1org.gulldesk.shapes/' "$file"
fi
exit 0
