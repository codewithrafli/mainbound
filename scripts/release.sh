#!/usr/bin/env bash
# Release Mainbound: bump version everywhere, commit, tag, push.
# Usage: bun run release 0.2.0
set -euo pipefail

VERSION="${1:?usage: bun run release <version>  (e.g. 0.2.0)}"
VERSION="${VERSION#v}"

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "error: version must be semver like 0.2.0" >&2
  exit 1
fi

if [[ -n "$(git status --porcelain)" ]]; then
  echo "error: working tree not clean — commit or stash first" >&2
  exit 1
fi

echo "→ bumping to v${VERSION}"

# package.json + tauri.conf.json
bun -e "
const fs = require('fs');
for (const file of ['package.json', 'src-tauri/tauri.conf.json']) {
  const json = JSON.parse(fs.readFileSync(file, 'utf8'));
  json.version = '${VERSION}';
  fs.writeFileSync(file, JSON.stringify(json, null, 2) + '\n');
}
"

# Cargo.toml (first version line in [package])
sed -i '' "0,/^version = \".*\"/s//version = \"${VERSION}\"/" src-tauri/Cargo.toml

# keep Cargo.lock in sync
(cd src-tauri && cargo update -p mainbound --quiet 2>/dev/null) || true

git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: release v${VERSION}"
git tag "v${VERSION}"
git push origin HEAD
git push origin "v${VERSION}"

echo ""
echo "✓ v${VERSION} tagged & pushed — GitHub Actions is building the release."
echo "  https://github.com/codewithrafli/mainbound/actions"
