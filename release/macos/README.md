# macOS packaging

ZeroWhats uses the system **WKWebView**, so no browser is bundled. The Tauri
bundler produces:

- `.app` — the application bundle (drag-to-Applications / "app image")
- `.dmg` — the distributable disk image

Build on a macOS runner (can't be cross-compiled from Linux). The release
workflow builds both `aarch64` (Apple Silicon) and `x86_64` (Intel):

```bash
bun install --frozen-lockfile
bun run tauri build --target aarch64-apple-darwin   # or x86_64-apple-darwin
# → src-tauri/target/<triple>/release/bundle/{macos,dmg}/
```

Code signing and notarization (`.github/workflows/release.yml`) activate automatically
once these repo secrets exist — builds stay ad-hoc-signed and unauthorized otherwise:

| Secret                       | Contents                                                                        |
| ---------------------------- | ------------------------------------------------------------------------------- |
| `APPLE_CERTIFICATE`          | Base64 of your Developer ID Application `.p12` (`base64 -i cert.p12 \| pbcopy`) |
| `APPLE_CERTIFICATE_PASSWORD` | Password used when exporting that `.p12`                                        |
| `KEYCHAIN_PASSWORD`          | Any password — used only for the CI run's temporary keychain                    |
| `APPLE_ID`                   | Apple ID email for notarization                                                 |
| `APPLE_PASSWORD`             | An app-specific password for that Apple ID (not your account password)          |
| `APPLE_TEAM_ID`              | Your team ID from the Apple Developer "Membership" page                         |
