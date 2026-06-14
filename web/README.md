# Rift download site

Static download page for Rift. Plain HTML/CSS/JS — no framework, no build step.

This deploys to **Cloudflare Pages** with the **build output dir set to `web/`** and
**no build command**; Pages serves the directory as-is. The "Download for Windows"
CTA and the version feed point at the Cloudflare R2 bucket — replace the
`pub-REPLACE.r2.dev` placeholders in `index.html` and `app.js` with the real bucket
hash once the bucket exists (see `docs/design/self-hosted-distribution-BUILD.md` §H).
