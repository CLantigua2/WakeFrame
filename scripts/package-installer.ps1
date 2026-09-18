param(
    [string]$Version = "0.1.0",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$isccPath = Join-Path ${env:ProgramFiles(x86)} "Inno Setup 6\ISCC.exe"

if (-not (Test-Path -LiteralPath $isccPath -PathType Leaf)) {
    $isccCommand = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    if ($isccCommand) {
        $isccPath = $isccCommand.Source
    }
}

if (-not (Test-Path -LiteralPath $isccPath -PathType Leaf)) {
    throw "Inno Setup compiler was not found. Install it once with: winget install JRSoftware.InnoSetup"
}

& (Join-Path $PSScriptRoot "package-portable.ps1") -Version $Version -SkipBuild:$SkipBuild
New-Item -ItemType Directory -Force (Join-Path $repoRoot "dist\installer") | Out-Null
& $isccPath (Join-Path $repoRoot "installer\WakeFrame.iss") /DAppVersion="$Version"

Write-Host "Installer written to $(Join-Path $repoRoot "dist\installer\WakeFrame-Setup.exe")"
