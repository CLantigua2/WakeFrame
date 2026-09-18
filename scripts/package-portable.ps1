param(
    [string]$Version = "0.1.0",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$stageDir = Join-Path $repoRoot "dist\WakeFrame"
$zipPath = Join-Path $repoRoot "dist\WakeFrame-Portable-$Version.zip"

if (-not $SkipBuild) {
    cargo build --workspace --release
}

New-Item -ItemType Directory -Force $stageDir | Out-Null
Copy-Item (Join-Path $repoRoot "target\release\wakeframe-agent.exe") $stageDir -Force
Copy-Item (Join-Path $repoRoot "target\release\wakeframe-ui.exe") $stageDir -Force
Copy-Item (Join-Path $repoRoot "target\release\wakeframe-player.exe") $stageDir -Force
Copy-Item (Join-Path $repoRoot "vendor\mpv\libmpv-2.dll") $stageDir -Force
Copy-Item (Join-Path $repoRoot "README.md") $stageDir -Force
Copy-Item (Join-Path $repoRoot "assets") $stageDir -Recurse -Force

if (Test-Path $zipPath) {
    Remove-Item $zipPath -Force
}

Compress-Archive -Path (Join-Path $stageDir "*") -DestinationPath $zipPath
Write-Host "Portable bundle written to $zipPath"
