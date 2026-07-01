# Security Policy

## Project status

ZeroWhats is **pre-1.0 and not yet audited**. It is built and maintained by a
single contributor in their spare time. Treat it as you would any young
open-source desktop client — useful, actively developed, but not something to
bet sensitive workflows on without doing your own review first. In particular,
today:

- **Releases are unsigned by default.** Code signing (macOS notarization,
  Windows Authenticode) is opt-in in CI and only activates if the maintainer
  configures certificate secrets (`.github/workflows/release.yml`). Until then,
  expect Gatekeeper/SmartScreen warnings — verify release checksums if that
  matters for your threat model.
- **No independent security audit has been performed.** The notes below
  describe the design's intent, not a verified guarantee.
- **The app-lock has no brute-force throttling.** `unlock` checks the Argon2id
  hash with no rate limit or lockout, so it only meaningfully helps against
  casual/opportunistic access to an already-unlocked machine, not a
  determined local attacker.
- **The config file (including the password hash) is a plain JSON file** in
  the OS app-config directory, not stored in the system keyring/Secret
  Service. Anyone with read access to that file (or a disk backup of it) gets
  the Argon2id hash to attack offline.

We still take reports seriously and fix what we can — see below.

## Supported Versions

Security fixes are applied to the latest release and the `main` branch.

## Reporting a Vulnerability

**Please do not open a public issue for security vulnerabilities.**

Report privately via one of:

- GitHub's [private vulnerability reporting](https://github.com/ZauJulio/ZeroWhats/security/advisories/new)
- Email: <zaujulio.dev@gmail.com>

Please include:

- A description of the issue and its impact
- Steps to reproduce (a proof of concept if possible)
- Affected version and platform (Linux/macOS/Windows)

You can expect an acknowledgement within a few days. We'll keep you updated on
the fix and coordinate disclosure once a patch is available.

## Scope & design notes

ZeroWhats is a thin client around WhatsApp Web; it stores no message content and
sends no telemetry. Some properties relevant to security reports:

- **WhatsApp-only navigation.** The app window only loads WhatsApp; other links
  are opened in the user's default browser (`src-tauri/src/window.rs`).
- **Remote-origin IPC is minimal and capability-scoped.** The remote WhatsApp
  page can never invoke app (`#[tauri::command]`) functions — those are only
  reachable from the local React windows (`src-tauri/src/commands.rs`). Its
  only Tauri-side grant is `capabilities/whatsapp-remote.json`: a handful of
  window controls (minimize/close/drag/toggle-maximize/fullscreen) plus
  emitting the fixed `zw://action`, `zw://unread`, `zw://notify`,
  `zw://open-external` events, which `main.rs` listens for explicitly. The
  broader `capabilities/default.json` set (dialog, notifications, arbitrary
  event listen) deliberately has no `remote` block, so none of it reaches the
  WhatsApp origin — only the local app windows (Settings/About/Shortcuts/Lock).
- **Local windows** run under a strict Content-Security-Policy
  (`src-tauri/tauri.conf.json`).
- **App lock** passwords are stored only as an Argon2id hash, in
  `config.json` — see the brute-force/keyring caveats in "Project status"
  above.

Findings that strengthen any of the above — including gaps in the capability
scoping itself — are very welcome.
