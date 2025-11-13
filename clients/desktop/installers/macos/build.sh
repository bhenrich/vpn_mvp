#!/usr/bin/env bash
set -euo pipefail

CONFIGURATION="${1:-release}"

echo "==> Building macOS desktop client (${CONFIGURATION})..."

pushd "$(dirname "${BASH_SOURCE[0]}")/../../ui" >/dev/null

if command -v pnpm >/dev/null 2>&1; then
  if [[ -f "pnpm-lock.yaml" ]]; then
    pnpm install --frozen-lockfile
  else
    pnpm install
  fi
else
  echo "pnpm is required to build the Tauri UI" >&2
  exit 1
fi

if [[ "${CONFIGURATION}" == "release" ]]; then
  pnpm tauri build
else
  pnpm tauri build --debug
fi

popd >/dev/null

echo "==> macOS DMG/notarization is environment-specific."
echo "    Integrate this script with your CI to:"
echo "      - Sign the app bundle."
echo "      - Create a DMG (e.g., via create-dmg or hdiutil)."
echo "      - Notarize with notarytool and staple the ticket."


