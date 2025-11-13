Linux installers (DEB/RPM/AppImage)
===================================

This directory contains scaffolding for packaging the desktop client on Linux.

For the MVP, the recommended approach is:

- Use the Tauri bundler to emit `.deb`, `.rpm`, and/or `.AppImage` artifacts.
- Wrap any platform-specific post-install steps (e.g., PolicyKit rules) in
  the OS package metadata as needed.

The helper script `build.sh` delegates to the Tauri bundler and can be wired
into CI for reproducible builds.


