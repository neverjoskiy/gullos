desktops().forEach(function (desktop) {
    if (desktop.wallpaperPlugin === "org.gulldesk.tower") {
        desktop.wallpaperPlugin = "org.gulldesk.shapes";
    }
});
