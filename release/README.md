# Packaging & Releases

Everything needed to package ZeroWhats for each platform. Built artifacts are
written to `release/_artifacts/` (git-ignored); the files here are the packaging
**definitions**, kept under version control.

| Target  | Format(s)                   | Definition                                         | Produced by                    |
| ------- | --------------------------- | -------------------------------------------------- | ------------------------------ |
| Linux   | `.AppImage`, `.deb`, `.rpm` | Tauri bundler config (`src-tauri/tauri.conf.json`) | `release/build-linux.sh` or CI |
| Linux   | AUR (`zerowhats-bin`)       | `aur/PKGBUILD`                                     | CI (`release.yml`)             |
| Windows | `.msi`, `.exe` (NSIS)       | Tauri bundler config                               | CI (`release.yml`)             |
| macOS   | `.dmg`, `.app`              | Tauri bundler config                               | CI (`release.yml`)             |

Shared Linux metadata lives in `linux/` (the `.desktop` entry and the AppStream
`.metainfo.xml`), available to distro packagers.

## Build locally

Linux installers (on a Linux machine with the system deps):

```bash
./release/build-linux.sh        # → release/_artifacts/*.{deb,rpm,AppImage}
```

Windows and macOS installers must be built on their own OS (the WebView is
provided by the system: WebView2 on Windows, WKWebView on macOS), so they're
produced by the GitHub Actions matrix rather than cross-compiled from Linux.

## Release (CI)

Push a tag (`vX.Y.Z`) to run `.github/workflows/release.yml`: it builds the
desktop matrix (deb/rpm/AppImage, msi/nsis, dmg/app) and attaches everything to
a draft GitHub Release. The AUR job is best-effort and needs its own credentials:

| Job | Secret                | Notes                                            |
| --- | --------------------- | ------------------------------------------------ |
| AUR | `AUR_SSH_PRIVATE_KEY` | deploy key registered on your AUR account        |
