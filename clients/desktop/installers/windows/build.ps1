Param(
    [string]$Configuration = "release"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "==> Building Windows desktop client ($Configuration)..." -ForegroundColor Cyan

# 1. Build the Tauri UI (bundled with Rust backend)
Push-Location "..\..\ui"
try {
    if (Test-Path "pnpm-lock.yaml") {
        pnpm install --frozen-lockfile
    }
    else {
        pnpm install
    }

    if ($Configuration -eq "release") {
        pnpm tauri build
    }
    else {
        pnpm tauri build --debug
    }
}
finally {
    Pop-Location
}

# 2. Build the desktop-service sidecar
Write-Host "==> Building desktop-service..." -ForegroundColor Cyan
cargo build -p desktop-service @(
    if ($Configuration -eq "release") { "--release" } else { "--debug" }
)

Write-Host "==> Windows packaging step (MSI/MSIX) is environment-specific." -ForegroundColor Yellow
Write-Host "    Integrate this script with WiX/MSIX tooling in CI to generate signed installers." -ForegroundColor Yellow


