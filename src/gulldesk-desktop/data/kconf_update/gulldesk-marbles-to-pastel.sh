#!/bin/sh
file="${XDG_CONFIG_HOME:-$HOME/.config}/kscreenlockerrc"
if [ -f "$file" ] && grep -q 'org\.gulldesk\.marbles' "$file"; then
  sed -i 's/^\([Ww]allpaper[Pp]lugin=\)org\.gulldesk\.marbles$/\1org.gulldesk.pastel/' "$file"
fi
exit 0
