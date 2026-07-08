# Rift download site

Static download page for Rift. Plain HTML/CSS/JS — no framework, no build step.

Deploys to **Cloudflare Pages** with the build output dir set to `web/` and no build
command; Pages serves the directory as-is. The "Download for Windows" CTA and the
version feed read the public Cloudflare R2 release bucket (the same feed the app's
self-updater uses).
