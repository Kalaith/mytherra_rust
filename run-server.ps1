# Minimal local run for the mytherra authority server (GDD 7).
#
# Builds and runs the server against the local MySQL. The world persists to the
# DB (GDD 6/8), so stopping and restarting resumes it rather than resetting.
#
# Prerequisites (XAMPP):
#   - MySQL running — the server auto-creates the `mytherra_rust` database and
#     migrates it on first start. Credentials come from mytherra-server/.env.
#   - Apache running only if you also serve the WebGL client locally (below).
#
# The clients are separate:
#   - WebGL (browser): deploy with `.\publish.ps1 -WebGLOnly`, then open
#     http://127.0.0.1/games/mytherra/ — each browser tab is its own guest deity.
#   - Native window: `cargo run -p mytherra` (also connects to this server).
# All of them point at this one server, so several at once exercise the shared
# world and concurrent deities.

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

# Surface the address clients must reach, straight from the single config source.
$cfg = Get-Content (Join-Path $PSScriptRoot "assets\data\game_config.json") -Raw | ConvertFrom-Json

Write-Host ""
Write-Host "Authority server : http://$($cfg.server_listen_addr)" -ForegroundColor Green
Write-Host "WebGL client     : http://127.0.0.1/games/mytherra/   (deploy with .\publish.ps1 -WebGLOnly)" -ForegroundColor Green
Write-Host "Native client    : cargo run -p mytherra" -ForegroundColor Green
Write-Host "Ctrl+C stops the server; the world is saved to MySQL and resumes on the next run." -ForegroundColor DarkGray
Write-Host ""

# `cargo run` rebuilds only if something changed, so this doubles as the build.
cargo run --release -p mytherra-server
if ($LASTEXITCODE -ne 0) { Write-Error "mytherra-server exited with $LASTEXITCODE"; exit 1 }
