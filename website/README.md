# vinext-starter

A clean full-stack starter running on
[vinext](https://github.com/cloudflare/vinext), with optional Cloudflare D1 and
Drizzle support.

## Prerequisites

- Node.js `>=22.13.0`

## Quick Start

```bash
pnpm install
pnpm run dev
pnpm run build
```

This starter does not use `wrangler.jsonc`.

## Included Shape

- edit site code under `app/`
- `.openai/hosting.json` declares optional Sites D1 and R2 bindings
- `vite.config.ts` simulates declared bindings for local development
- `db/schema.ts` starts intentionally empty
- `examples/d1/` contains an optional D1 example surface
- `drizzle.config.ts` supports local migration generation when needed

## Useful Commands

- `pnpm run dev`: start local development
- `pnpm run build`: verify the vinext build output
- `pnpm test`: build the site and verify its rendered HTML
- `pnpm run deploy`: build and deploy to Cloudflare Workers
- `pnpm run deploy:dry-run`: validate a Cloudflare deployment without publishing
- `pnpm run db:generate`: generate Drizzle migrations after schema changes

## Learn More

- [vinext Documentation](https://github.com/cloudflare/vinext)
- [Drizzle D1 Guide](https://orm.drizzle.team/docs/get-started/d1-new)

## Desktop download releases

The localized `#download` section in `app/page.tsx` points to the public binary-only
GitHub prerelease `desktop-preview-20260928` (`0.1.0-preview.1`). Desktop source and
build workflows remain in the private `termleaf-apps` repository. Publish and
verify all four installers and `SHA256SUMS.txt` before deploying links to a new
release. Keep the version, tag, filenames, signing caveat and rendered HTML tests
in sync when replacing a preview.

The terminal installation commands and `releases/latest` link remain separate.
Desktop prereleases must not become the repository's latest stable terminal
release. All macOS, Windows and Linux choices are rendered for every locale;
there is no user-agent or architecture guessing.

## Deployment CLI compatibility

The deploy scripts pin `cf@0.6.0`, which reads the experimental Build Output format
produced by the locked `@cloudflare/vite-plugin@1.50.0`. Keep build and deploy on
that same CLI version. An unversioned `npx cf` currently resolves to a newer format
that requires a root config and renamed Worker config files, so it cannot read
this build output. This pin adds no project dependency or output-format adapter.

Before publishing, run `pnpm run deploy:dry-run` end to end; it builds, prepares
the SSR modules and validates the bundle and static assets without uploading.
Upgrade the CLI and build plugin together only after this check succeeds.
