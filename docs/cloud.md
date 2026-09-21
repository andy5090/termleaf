# Termleaf Cloud development

Termleaf Cloud adds Google sign-in, subscription entitlement, connected devices
and explicit document transfer to the existing desktop and mobile writing UI.
Users open **Cloud → Continue with Google**. They never configure a server URL,
password or API key. The app opens the system browser; after Google sign-in the
browser asks for the six-digit connection code displayed in the app. The app
then connects automatically. Cancel leaves all local writing available.

The service is developed in the separate **private** `termleaf-cloud` repository:
Cloudflare Workers for HTTP, D1 for accounts/versions/subscriptions and a private
R2 bucket for document bodies. Server code, provider secrets and admin tooling
must not be copied into this public repository. Railway remains an alternative;
this implementation uses Cloudflare directly.

## Configuration belongs to the operator

Desktop builds use `VITE_TERMLEAF_CLOUD_URL`; Expo builds use
`EXPO_PUBLIC_TERMLEAF_CLOUD_URL`. These contain only the public service origin,
not secrets. Set the origin at build time. Release builds without one show that
Cloud is not available yet; they do not ask the user for a server. Desktop dev
uses `http://127.0.0.1:8787` when no override is set. HTTP is allowed only on
loopback; physical-device development needs an HTTPS service.

The private server README covers Google Web application OAuth credentials,
registered callback URL, Workers/D1/R2 setup, migrations and Polar configuration.
Google and Polar secrets stay exclusively on the server. Real Google OAuth
credentials and a registered callback are required for a real Google account
login. There is no production mock-login switch. Tests replace provider HTTP
responses inside the isolated local test runtime only.

Polar is the first billing provider. No product or price has been selected yet.
Until those exist, the operator can grant local trial entitlements without
charging anyone. Desktop can open provider checkout and the customer portal;
mobile currently consumes account entitlements without an in-app purchase link.
Store distribution and payment-policy review remain release work.

## Document behavior

- Upload sends the active document. Open copy imports a new local document;
  existing unsaved text is never overwritten by a download.
- Re-uploading that document during the same signed-in app session updates its
  known remote version. A conflicting version returns an error; open the latest
  copy and merge locally before uploading it. No silent last-writer-wins retry.
- Bindings and access tokens currently stay in memory. After restart/sign-out,
  sign in again and open the cloud copy to resume updating that remote file.
  Uploading an unbound local document creates a separate cloud file.
- Transfers are explicit, not automatic background folder synchronization.
  Failed requests leave local writing intact. A connection failure after a
  server commit may still have saved the remote version; refresh before retrying.
- An expired subscription blocks new uploads while retaining read/export and
  deletion access. The server enforces quotas and per-device session revocation.
- Default logical quota is 100 MiB and max document size is 2 MiB, configurable
  by the operator. These are development defaults, not a published paid plan.
- This version does not provide end-to-end encryption, attachment syncing,
  shared editing, a version-history UI or terminal/Termux cloud commands.

## Public code boundaries

`packages/cloud` holds the transport, response validation, OAuth polling,
version bindings and React state shared by both apps. Native desktop imports
use the existing Rust editor and create a new buffer. Mobile imports reuse the
existing notebook persistence. No second editor or server SDK is embedded.

The native bridge restricts checkout links to Polar HTTPS and sign-in links to
the service's authentication path. The shared client requires the configured
service origin, refuses redirects and keeps the poll secret out of browser URLs.

## Verification

Run `pnpm test`, `pnpm typecheck`, `pnpm lint`, `pnpm build:desktop`, and
`pnpm build:mobile`. Native bridge checks are listed in [apps.md](apps.md).
The private repository separately tests auth isolation, signed provider
callbacks, replay protection, quota races, conflicts, garbage collection and
D1/R2 backup restoration using the local Workers runtime.

Real Google consent and Polar checkout/webhook delivery require operator
credentials and remain distinct from fixture-based tests. Native Android/iOS
runtime verification and public production rollout are not established by an
Expo bundle export.
