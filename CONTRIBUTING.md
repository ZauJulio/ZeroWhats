# Contributing to ZeroWhats

Thanks for your interest in improving ZeroWhats! This guide covers everything you
need to get productive.

## Prerequisites

- [Bun](https://bun.sh) (package manager + JS runtime)
- [Rust](https://rustup.rs) (stable toolchain)
- The [Tauri v2 system dependencies](https://v2.tauri.app/start/prerequisites/)
  for your OS (on Linux: `libwebkit2gtk-4.1`, `libgtk-3`,
  `libayatana-appindicator3`, `librsvg2`, `patchelf`)

## Getting started

```bash
git clone https://github.com/ZauJulio/ZeroWhats.git
cd ZeroWhats
bun install
bun run tauri dev      # hot-reloading dev build
```

## Project layout

See the [Architecture section of the README](README.md#architecture). In short:

- `src/` — the React frontend (frameless secondary windows), layered into
  `lib/` (IPC, theme, i18n), `ui/` (components) and `screens/`.
- `src-tauri/src/` — the Rust backend, one module per concern (`window`, `lock`,
  `notification`, `tray`, `commands`, `config`), plus `web/*.js` (scripts
  injected into the WhatsApp page).

> **Important:** the WhatsApp window runs at a remote origin and **cannot invoke
> app commands** — it talks to the backend via events (`zw://*`). See the
> README's "remote-origin IPC" note before touching the titlebar or tray.

## Code style & checks

```bash
bun run lint           # oxlint
bun run format         # oxfmt (writes changes)
bun run format:check   # oxfmt (CI check)
cd src-tauri && cargo fmt && cargo clippy
```

- TypeScript/React is formatted by **oxfmt** and linted by **oxlint**.
- Rust is formatted by `cargo fmt`; keep `cargo build` warning-free.
- CSS lives in `*.module.css` files using native nesting (Lightning CSS lowers
  it for the target webviews). Global tokens/resets are in `src/styles.css`.

### Git hooks

Hooks live in `.githooks/` and install automatically on `bun install` (via the
`prepare` script, which points `core.hooksPath` at the folder). To (re)install
manually: `bun run hooks:install`.

- **pre-commit** — formats staged files (`oxfmt` for TS/JS, `cargo fmt` for Rust)
  and lints staged TS with `oxlint`, then re-stages them. Stage whole files; a
  file with both staged and unstaged hunks is re-staged in full after formatting.
- **pre-push** — runs the full gate: `oxlint`, `oxfmt --check`, `bun run build`
  (tsc + vite) and `cargo check`.

Skip a hook for a one-off with `git commit --no-verify` / `git push --no-verify`.

## Commits & pull requests

- Use clear, present-tense commit messages (Conventional Commits encouraged:
  `feat:`, `fix:`, `refactor:`, `docs:`…).
- Keep PRs focused; describe the change and how you tested it.
- Make sure `bun run build` and `cd src-tauri && cargo build` both pass.
- By contributing you agree your work is licensed under the project's
  [MIT License](LICENSE).

## Reporting bugs & ideas

Open an [issue](https://github.com/ZauJulio/ZeroWhats/issues) with steps to
reproduce, your OS/desktop environment, and logs where relevant. For security
issues, follow [SECURITY.md](SECURITY.md) instead of opening a public issue.
