// Fetches the Velopack release feed to show the current version. Graceful
// fallback to a static label if the fetch fails (offline / CORS / pre-launch).
// HUMAN: the feed lives at the R2 bucket root — same host as the Setup.exe CTA.
(function () {
  "use strict";

  var FEED_URL = "https://pub-4fb26c0fc8df484488e4415f112f2d28.r2.dev/releases.win.json";
  var FALLBACK = "Latest release";

  function setVersion(text) {
    var line = document.getElementById("version-line");
    var foot = document.getElementById("footer-version");
    if (line) line.textContent = text;
    if (foot) foot.textContent = "Rift " + text;
  }

  // Velopack releases.<channel>.json shape: { Assets: [{ Version, ... }] }.
  // Pick the highest version string present; tolerate schema drift.
  function pickVersion(feed) {
    if (!feed) return null;
    var assets = feed.Assets || feed.assets || [];
    var versions = assets
      .map(function (a) { return a && (a.Version || a.version); })
      .filter(Boolean);
    if (!versions.length) return null;
    versions.sort();
    return versions[versions.length - 1];
  }

  try {
    fetch(FEED_URL, { cache: "no-store" })
      .then(function (r) { return r.ok ? r.json() : null; })
      .then(function (feed) {
        var v = pickVersion(feed);
        setVersion(v ? "v" + v : FALLBACK);
      })
      .catch(function () { setVersion(FALLBACK); });
  } catch (e) {
    setVersion(FALLBACK);
  }
})();
