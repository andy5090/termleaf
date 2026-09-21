# Desktop GUI and mobile development

The desktop app is a native GUI port of Termleaf using Tauri and React. Its Rust
backend uses the same `termleaf` library as the terminal binary: the editor,
Korean/Japanese composers, Galmuri bitmap glyphs, themes, configuration, language
packs and typewriter sounds. It opens and saves ordinary text files.

The writing surface follows the terminal layout: large cursor-side pixel text,
a monospaced document, and compact status and command rows along the bottom.
OS input methods, mouse selection and clipboard input use the webview's text
control. Termleaf's optional live input modes use the existing Rust composers.

The Expo mobile app now follows the same writing interface: a cursor-following
pixel display, one monospaced document and compact status/command rows. It uses
the terminal's exact Galmuri glyph data (ASCII, Korean and Japanese) and four
palettes. It remains experimental: native OS input and the existing local
notebook storage are used; Rust live composition, typing sounds, desktop file
access are not implemented on mobile. Cloud document transfers are available as
described in [Cloud development](cloud.md). Android and iOS remain
in scope, along with Windows, macOS and Linux on desktop. A configured target
is not a claim that its native runtime has been verified.

## Run locally

Use Node.js 24 and pnpm 10.31.0 from the repository root:

```sh
pnpm install --frozen-lockfile
pnpm desktop
```

Tauri needs each platform's native build tools. See its
[prerequisites](https://v2.tauri.app/start/prerequisites/). Linux also needs the
ALSA development headers used by Termleaf's existing audio engine; the app CI
workflow installs these with the webview prerequisites.

On a Mac with Command Line Tools selected and full Xcode installed, use a
per-command override:

```sh
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer pnpm desktop
```

`pnpm --filter @termleaf/desktop dev` starts the web frontend alone. Editing
requires the native Tauri backend; the browser does not silently create another
notebook in local storage.

For the experimental mobile app:

```sh
pnpm mobile
```

Use Expo's Android/iOS launch options and an Expo Go version compatible with the
pinned SDK, or a development build. See the
[Expo environment guide](https://docs.expo.dev/get-started/set-up-your-environment/).

## Desktop files and preferences

Open and Save As accept ordinary file paths. First save asks for a filename;
a name without an extension defaults to `.md`. Existing terminal documents can
be edited directly. Opening a file adds it to the open-document list; opening
that path again selects its existing buffer. Switching documents retains unsaved
text and cursor positions. Failed operations leave documents available.

The collapsible left sidebar contains **New writing**, open documents, an active
selection and unsaved markers. Ctrl/Cmd+N creates a blank document; Ctrl/Cmd+B
collapses or expands the sidebar. Focus mode hides it with the other controls.
Closing checks every open document. **Save all** saves named documents and asks
for a filename for each changed unnamed draft; cancellation or a failed save
keeps the app open. Autosave applies to the active named document. Open sessions
are kept in memory; the list and unnamed drafts are not restored after restart.

The desktop app shares Termleaf's preferences and installed language packs with
the terminal app. Configuration lives at `~/.config/termleaf/config`; language
packs use `TERMLEAF_DATA_HOME`, then `XDG_DATA_HOME/termleaf`, then
`~/.local/share/termleaf`. On Windows, `USERPROFILE` supplies the home directory
when `HOME` is absent. Existing terminal environment overrides still apply.

The rejected desktop notebook prototype's `library.json` is left untouched in
the app data directory. The new desktop GUI does not read or overwrite it.
If that prototype contains writing you need, retain the file for recovery.

The mobile app retains `@termleaf/notebook` and its local
versioned notebook format. It keeps alternating `notebook-a.json` and
`notebook-b.json` snapshots in Expo's document directory. This is separate from
both the terminal files and desktop GUI.

On mobile, **Menu** opens a left overlay drawer with **New writing** and the
local document list. Creating or selecting a draft closes the drawer. The menu
remains available in focus mode. Close, tapping the backdrop, Android Back and
Escape on web dismiss it. The drawer uses 86% of phone width, capped at 360,
leaving the writing surface full width when closed. It reuses the existing
notebook persistence. The former title/body cards are replaced by a single
writing surface; existing titles remain document names, editable by tapping
the name above the editor. Body text is not migrated or rewritten.

Mobile commands include save, focus, theme, pixel size, line spacing, big text
and page width. Scroll the bottom command row to reach additional controls.
The pixel preview fits the available width and follows the native keyboard's
cursor. Focus keeps Menu and Exit focus reachable. Visual preferences currently
last for the app session; documents and their names persist locally.

`node scripts/generate-mobile-glyphs.mjs` regenerates the mobile bitmap data
from the existing terminal assets. `pnpm test` checks that the generated data
and bundled OFL license match the source. The full font license is also
available from mobile Help.

## Keyboard and mouse controls

| Control | Action |
| --- | --- |
| Ctrl/Cmd+N, Ctrl/Cmd+B | New writing, toggle sidebar |
| Ctrl/Cmd+O, Ctrl/Cmd+S | Open, save |
| Ctrl/Cmd+Shift+S, F12 | Save as |
| F1 | Help |
| F2 / Shift+F2 | Next / previous installed input mode |
| F3, F4 | Focus mode, big pixel text |
| F5 / Shift+F5 | Page width / line spacing |
| F6 | Theme: paper, night, xt, amber |
| F7 / F8 | Smaller / larger pixel text |
| F9 | Language packs and interface language |
| F10 / F11 | Sound settings / sound profile |
| Ctrl/Cmd+K | Hiragana / katakana in live Japanese input |

The bottom controls expose commands without requiring function keys. OS-level
keyboard shortcuts may require an Fn key depending on the keyboard's settings.

## Development checks

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-features --locked
pnpm test
pnpm typecheck
pnpm lint
pnpm build:desktop
pnpm build:mobile
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked
```

The app workflow defines native desktop checks for Windows, macOS and Linux.
It does not publish apps or require signing keys. The website has its own package
manifest and lockfile and remains outside this workspace.

## Verification and remaining work

Verified on 2026-09-15:

| Check | Evidence |
| --- | --- |
| Shared Rust engine and terminal | 115 tests passed, 2 optional model tests ignored; rustfmt and Clippy with warnings denied passed |
| Desktop Rust bridge | 9 tests passed: Unicode caret offsets, live composition, Shift+Tab, file errors, `.md` defaults, folder navigation, language removal and real glyph/theme data; rustfmt and Clippy passed |
| JavaScript workspace | 16 tests passed; typecheck, ESLint and desktop production build passed |
| macOS native app | Actual Tauri window: Korean/Japanese/emoji file save and reopen, first-save `.md` default, idle autosave, Cancel/Save/Discard guards, failed-save recovery and folder navigation verified |
| Native input | macOS two-set Korean input, coexistence with Live Korean, shared-engine `ㅎ → 하 → 한`, and installed Akaza model conversion `nihongo → 日本語` verified |
| Browser UI regression | A test-only bridge fixture verified IPC ordering, OS composition isolation/commit, the WebKit IME key guard and focus controls; screenshot compared with the terminal reference |

Browser fixtures are verification artifacts, not a shipped browser editor or
proof of native storage. Windows/Linux CI is configured but has not been run
remotely for this change. Their native runtime remains unverified. The two
ignored core tests require explicit optional model-test setup; the installed
Japanese model was exercised in the macOS native GUI.

Remaining release work includes Windows/Linux native runtime checks, signed
installers, distribution, updates, physical device/IME coverage and mobile engine integration. Automatic background synchronization and end-to-end encryption remain future
work. Account, subscription and explicit cloud transfer support uses the separate
private server described in [Cloud development](cloud.md).

### Navigation verification — 2026-09-16

- Desktop bridge: 16 tests passed, including Unicode/caret preservation,
  composition commit, path deduplication, active-only save and save-as collision
  protection. Rustfmt and Clippy passed.
- Workspace: 19 JavaScript tests, typecheck, ESLint and desktop production build
  passed. Expo exported Android, iOS and web bundles successfully.
- Actual macOS Tauri window: created two Unicode drafts, switched without losing
  text, collapsed/expanded the sidebar, and saved both through successive Save
  As dialogs before the window closed. Relaunched the app afterward.
- Desktop browser fixture: delayed navigation blocks old-buffer edits; a clean
  active draft cannot bypass inactive dirty-document protection; failed Save all
  leaves the window open and retry completes saving.
- Mobile web: New writing, selection, Unicode persistence after reload, focus
  menu, Escape and backdrop dismissal passed. Screenshots checked at 320, 390
  and 768 pixels wide. Native iOS/Android drawer, hardware Back and keyboard/safe
  area behavior still need device verification; bundle export is not a runtime
  test. No dependencies were added.

Verification artifacts are in the ignored `output/playwright/app-navigation/`
directory. This machine's current Xcode app requires license acceptance, so
native Rust verification used the already-installed Command Line Tools via
`DEVELOPER_DIR=/Library/Developer/CommandLineTools` without changing global settings.

### Mobile interface alignment — 2026-09-16

The card-based mobile editor was replaced after navigation alone did not meet
the intended desktop/terminal interface. Verification includes:

- All 18,070 generated glyphs checked against the terminal's binary font sources;
  current-line cursor slicing tested with Hangul, Japanese and surrogate pairs.
- Existing mobile documents and names retained, single editor confirmed,
  new/switch/rename/save/reload exercised with Unicode text in Expo web.
- Four palettes, size and spacing controls, focus entry/exit, menu in focus,
  backdrop/Escape dismissal and bundled license verified in the browser.
- Focused and unfocused screenshots compared with the desktop and terminal at
  phone (320/390) and tablet (768) widths. This verifies layout, not native IME
  or Android Back. Android and iOS export succeeded; device runtime verification
  remains outstanding on this machine.

### Cloud integration — 2026-09-22

- Desktop and mobile now expose Google sign-in from Cloud, with no user-facing
  server/password form. Provider credentials remain server-side.
- Workspace: 30 JavaScript tests, typecheck, ESLint, desktop build and Expo
  Android/iOS/web export passed. Desktop Rust bridge: 19 tests, rustfmt and
  Clippy passed.
- Shared client exercised two real local Worker sessions against D1/R2:
  Unicode transfer, optimistic conflict, device revocation and tombstones.
- Browser provider fixtures exercised Google waiting/cancel/automatic completion,
  document import and conflict preservation. Mobile layouts checked at 320/390
  pixels; desktop overlay checked against the existing Termleaf interface.
- Actual macOS Tauri exercised the Cloud button and real local API configuration
  error path. Native cloud import is covered by the Rust bridge test. Browser
  bridge fixtures do not establish successful native Google login.
- Private server: 32 tests including actual local Workers runtime, Google RSA/JWKS
  fixtures, browser pairing/CSRF, one-time claims/cancellation, signed Polar
  webhooks, quotas, collection and backup restoration. Build dry-run passed.
- No hosted rollout, actual Google consent, real Polar payment or native mobile
  device test is claimed. See [Cloud development](cloud.md) for operator setup,
  explicit-transfer limitations and release work.
