# ~/.profile for gull (Bourne/POSIX shell login fallback)
[ -f /etc/profile.d/gullos-env.sh ] && . /etc/profile.d/gullos-env.sh

if [ -z "${WAYLAND_DISPLAY:-}" ] && [ "$(tty 2>/dev/null || true)" = "/dev/tty1" ]; then
    exec /usr/bin/gull-session
fi
