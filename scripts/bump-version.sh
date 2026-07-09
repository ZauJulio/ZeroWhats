#!/usr/bin/env bash
set -euo pipefail

# Usage: ./scripts/bump-version.sh <new-version>
# Example: ./scripts/bump-version.sh 1.4.0
#
# Updates the version in every file that carries it, then prints a summary.

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <new-version>"
  echo "Example: $0 1.4.0"
  exit 1
fi

NEW="$1"

if ! [[ "$NEW" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Error: version must be semver (e.g. 1.4.0), got '$NEW'"
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Read current version from the single source of truth
OLD=$(node -p "require('./package.json').version")

if [[ "$OLD" == "$NEW" ]]; then
  echo "Already at version $NEW — nothing to do."
  exit 0
fi

echo "Bumping $OLD → $NEW"
echo ""

bump() {
  local file="$1"
  local pattern="$2"
  local replacement="$3"

  if [[ ! -f "$file" ]]; then
    echo "  SKIP  $file (not found)"
    return
  fi

  if grep -q "$pattern" "$file"; then
    sed -i "s|$pattern|$replacement|g" "$file"
    echo "  OK    $file"
  else
    echo "  SKIP  $file (pattern not found)"
  fi
}

# --- Core manifests (source of truth) ---
bump "package.json" \
  "\"version\": \"$OLD\"" \
  "\"version\": \"$NEW\""

bump "src-tauri/Cargo.toml" \
  "^version = \"$OLD\"" \
  "version = \"$NEW\""

bump "src-tauri/tauri.conf.json" \
  "\"version\": \"$OLD\"" \
  "\"version\": \"$NEW\""

# --- Release / packaging ---
bump "release/aur/PKGBUILD" \
  "pkgver=$OLD" \
  "pkgver=$NEW"

bump "release/snap/snapcraft.yaml" \
  "version: \"$OLD\"" \
  "version: \"$NEW\""

bump "release/windows/winget/ZauJulio.ZeroWhats.yaml" \
  "PackageVersion: $OLD" \
  "PackageVersion: $NEW"

bump "release/windows/winget/ZauJulio.ZeroWhats.locale.en-US.yaml" \
  "PackageVersion: $OLD" \
  "PackageVersion: $NEW"

bump "release/windows/winget/ZauJulio.ZeroWhats.installer.yaml" \
  "PackageVersion: $OLD" \
  "PackageVersion: $NEW"

bump "release/windows/winget/ZauJulio.ZeroWhats.installer.yaml" \
  "download/v$OLD/" \
  "download/v$NEW/"

bump "release/windows/winget/ZauJulio.ZeroWhats.installer.yaml" \
  "ZeroWhats_${OLD}_" \
  "ZeroWhats_${NEW}_"

# --- Update Cargo.lock ---
if [[ -f "src-tauri/Cargo.lock" ]]; then
  (cd src-tauri && cargo update -p zerowhats --quiet 2>/dev/null) || true
  echo "  OK    src-tauri/Cargo.lock"
fi

echo ""
echo "Done. Don't forget to:"
echo "  1. Add a new <release> entry in release/linux/com.zaujulio.zerowhats.metainfo.xml"
echo "  2. git add -A && git commit -m 'chore(release): bump to v$NEW'"
echo "  3. git tag v$NEW && git push origin main --tags"
