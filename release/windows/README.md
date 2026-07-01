# Windows packaging

ZeroWhats uses the **WebView2** runtime that ships with Windows 10/11, so the
installer is small and needs no bundled browser. The Tauri bundler produces both
formats from `src-tauri/tauri.conf.json`:

- `.msi` (WiX) — per-machine installer
- `.exe` (NSIS) — per-user installer with optional WebView2 bootstrapper

Build on a Windows runner (can't be cross-compiled from Linux):

```powershell
bun install --frozen-lockfile
bun run tauri build            # → src-tauri/target/release/bundle/{msi,nsis}/
```

Code signing (`.github/workflows/release.yml`) activates automatically once these
repo secrets exist — builds stay unsigned otherwise:

| Secret                         | Contents                                                                  |
| ------------------------------ | ------------------------------------------------------------------------- |
| `WINDOWS_CERTIFICATE`          | Base64 of your code-signing `.pfx` (`certutil -encode cert.pfx cert.b64`) |
| `WINDOWS_CERTIFICATE_PASSWORD` | Password used when exporting that `.pfx`                                  |

CI imports the certificate and writes its thumbprint into
`src-tauri/tauri.windows.conf.json` (`bundle.windows.certificateThumbprint`) right
before the build, so no manual thumbprint lookup is needed.
