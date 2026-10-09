/*
 * First-paint theme bootstrap.
 *
 * Loaded synchronously from <head> (it is an external script because the app's Content
 * Security Policy forbids inline scripts), so the saved theme is on <html> before anything is
 * painted. Without it, a light-theme user would see a flash of the dark default on every start
 * until the configuration has been loaded.
 *
 * It only restores what `src/lib/theme/apply.ts` stored in localStorage (already validated) and
 * re-checks it conservatively, because storage is not a trusted source: a class name of the
 * form `theme-*`, and custom properties whose values contain only plain color/length/font
 * characters. Anything else is ignored; the app applies the real theme once it has loaded.
 */
(function () {
  try {
    var raw = window.localStorage.getItem("ntd.theme.v1");
    if (!raw) return;
    var saved = JSON.parse(raw);
    var root = document.documentElement;

    if (typeof saved.themeClass !== "string" || !/^theme-[a-z0-9-]{1,40}$/.test(saved.themeClass)) return;
    root.classList.add(saved.themeClass);

    var tokens = saved.tokens;
    if (!tokens || typeof tokens !== "object") return;

    var allowedValue = /^[#a-zA-Z0-9 .,%()\/'"_-]{1,200}$/;
    var applied = [];
    var names = Object.keys(tokens);
    for (var i = 0; i < names.length && i < 200; i++) {
      var name = names[i];
      var value = tokens[name];
      if (!/^--[a-z0-9-]{1,60}$/.test(name)) continue;
      if (typeof value !== "string" || !allowedValue.test(value) || /url\s*\(/i.test(value)) continue;
      root.style.setProperty(name, value);
      applied.push(name);
    }

    if (applied.length > 0 && (saved.scheme === "dark" || saved.scheme === "light")) {
      root.style.setProperty("color-scheme", saved.scheme);
    }
    // Lets the app clear exactly these properties when it applies the real theme.
    root.setAttribute("data-theme-tokens", applied.join(" "));
  } catch (e) {
    /* storage unavailable or corrupt: the app applies the theme once loaded */
  }
})();
