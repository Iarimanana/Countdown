# Builds the Windows installer on Windows: dist\Countdown-Setup-<version>.exe
# Needs Rust (https://rustup.rs) with the default MSVC setup.
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.*)"' | Select-Object -First 1).Matches.Groups[1].Value

# The installer embeds the app, so the app is built first.
cargo build --release -p countdown
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo build --release -p countdown-setup
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

New-Item -ItemType Directory -Force dist | Out-Null
Copy-Item target\release\countdown-setup.exe "dist\Countdown-Setup-$version.exe"
Write-Host "Installer ready: dist\Countdown-Setup-$version.exe"
