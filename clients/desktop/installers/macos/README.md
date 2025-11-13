macOS installers (DMG + notarization)
=====================================

This directory contains scaffolding for packaging the desktop client as a
signed, notarized macOS app distributed via DMG.

The recommended approach is:

- Use the Tauri bundler to produce a universal (`x86_64` + `arm64`) app bundle.
- Wrap the app into a DMG image.
- Submit the app for notarization using `notarytool` and staple the ticket.

The helper script `build.sh` assumes:

- Xcode command-line tools are installed.
- An Apple Developer ID certificate is available in the keychain.
- Environment variables for signing/notarization are set by CI.


