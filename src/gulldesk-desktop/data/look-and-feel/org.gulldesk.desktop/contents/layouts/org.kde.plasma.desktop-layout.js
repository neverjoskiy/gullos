var chosen = String(ConfigFile("gulldesk-setup-style", "Style").readEntry("wallpaper"));
var wallpaper = /^org\.gulldesk\.[a-z]+$/.test(chosen) ? chosen : "org.gulldesk.shapes";
var desktops = desktopsForActivity(currentActivity());
for (var i = 0; i < desktops.length; i++) {
    desktops[i].wallpaperPlugin = wallpaper;
}

loadTemplate("org.kde.plasma.desktop.defaultPanel");
if (String(ConfigFile("gulldesk-shellrc", "Layout").readEntry("style")) !== "classic") {
    loadTemplate("org.gulldesk.desktop.statusBar");
}
