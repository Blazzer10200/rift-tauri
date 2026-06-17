# Rift download site

Static download page for Rift. Plain HTML/CSS/JS — no framework, no build step.

This deploys to **Cloudflare Pages** with the **build output dir set to `web/`** and
**no build command**; Pages serves the directory as-is. The "Download for Windows"
CTA and the version feed point at the Cloudflare R2 bucket at
`https://pub-4fb26c0fc8df484488e4415f112f2d28.r2.dev` (the `rift-releases` public
dev URL). Setup arc: `git log -- docs/design/self-hosted-distribution-BUILD.md`.
