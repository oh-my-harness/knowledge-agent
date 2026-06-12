param(
    [string]$OutputDir = "dist\knowledge-agent"
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$packageRoot = Join-Path $repoRoot $OutputDir
$archivePath = "$packageRoot.zip"
$webDist = Join-Path $repoRoot "web\dist"
$releaseExe = Join-Path $repoRoot "target\release\knowledge-agent.exe"

Push-Location $repoRoot
try {
    npm --prefix web install
    npm --prefix web run build
    cargo build --release -p knowledge-agent-cli

    if (Test-Path $packageRoot) {
        Remove-Item -LiteralPath $packageRoot -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $packageRoot | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path $packageRoot "web") | Out-Null

    Copy-Item -LiteralPath $releaseExe -Destination (Join-Path $packageRoot "knowledge-agent.exe")
    Copy-Item -LiteralPath $webDist -Destination (Join-Path $packageRoot "web\dist") -Recurse
    Copy-Item -LiteralPath (Join-Path $repoRoot "README.md") -Destination (Join-Path $packageRoot "README.md")

    if (Test-Path $archivePath) {
        Remove-Item -LiteralPath $archivePath -Force
    }
    Compress-Archive -LiteralPath $packageRoot -DestinationPath $archivePath

    Write-Host "Package written to $packageRoot"
    Write-Host "Archive written to $archivePath"
    Write-Host "Run: .\knowledge-agent.exe serve <obsidian-vault>"
    Write-Host "Verify: .\scripts\verify-package.ps1"
    Write-Host "Install for current user: .\scripts\install-user.ps1"
}
finally {
    Pop-Location
}
