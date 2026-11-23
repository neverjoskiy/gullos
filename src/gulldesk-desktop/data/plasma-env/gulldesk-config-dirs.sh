case ":${XDG_CONFIG_DIRS:-}:" in
    *:/usr/share/gulldesk/xdg:*) ;;
    *) export XDG_CONFIG_DIRS="/usr/share/gulldesk/xdg:${XDG_CONFIG_DIRS:-/etc/xdg}" ;;
esac
