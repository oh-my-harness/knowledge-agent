param(
    [string]$PackageDir = "dist\knowledge-agent",
    [string]$InstallDir = "$env:LOCALAPPDATA\KnowledgeAgent",
    [switch]$NoPath
)

$ErrorActionPreference = "Stop"

function Get-SafeFullPath {
    param([string]$Path)

    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $rootPath = [System.IO.Path]::GetPathRoot($fullPath)
    $trimmedFullPath = $fullPath.TrimEnd('\')
    $trimmedRootPath = $rootPath.TrimEnd('\')

    if ($trimmedFullPath -eq $trimmedRootPath) {
        throw "InstallDir cannot be a drive root: $fullPath"
    }

    if ($trimmedFullPath.Length -lt 10) {
        throw "InstallDir is too short: $fullPath"
    }

    return $fullPath
}

function Test-PathEntryEquals {
    param(
        [string]$Entry,
        [string]$Target
    )

    if ([string]::IsNullOrWhiteSpace($Entry)) {
        return $false
    }

    try {
        $entryFullPath = [System.IO.Path]::GetFullPath($Entry.Trim().TrimEnd('\'))
        $targetFullPath = [System.IO.Path]::GetFullPath($Target.Trim().TrimEnd('\'))
        return $entryFullPath -ieq $targetFullPath
    }
    catch {
        return $false
    }
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$packageRoot = Resolve-Path (Join-Path $repoRoot $PackageDir)
$installRoot = Get-SafeFullPath $InstallDir

$packageExe = Join-Path $packageRoot "knowledge-agent.exe"
$packageWebIndex = Join-Path $packageRoot "web\dist\index.html"

if (-not (Test-Path $packageExe)) {
    throw "Package executable not found: $packageExe. Run .\scripts\package.ps1 first."
}

if (-not (Test-Path $packageWebIndex)) {
    throw "Packaged Web UI not found: $packageWebIndex. Run .\scripts\package.ps1 first."
}

if (Test-Path $installRoot) {
    Remove-Item -LiteralPath $installRoot -Recurse -Force
}

New-Item -ItemType Directory -Force -Path $installRoot | Out-Null
Get-ChildItem -LiteralPath $packageRoot | Copy-Item -Destination $installRoot -Recurse -Force

$pathUpdated = $false
if (-not $NoPath) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $pathEntries = @()
    if (-not [string]::IsNullOrWhiteSpace($userPath)) {
        $pathEntries = $userPath -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    }

    $alreadyInPath = $false
    foreach ($entry in $pathEntries) {
        if (Test-PathEntryEquals -Entry $entry -Target $installRoot) {
            $alreadyInPath = $true
            break
        }
    }

    if (-not $alreadyInPath) {
        $newUserPath = (($pathEntries + $installRoot) -join ';')
        [Environment]::SetEnvironmentVariable("Path", $newUserPath, "User")
        $env:Path = "$env:Path;$installRoot"
        $pathUpdated = $true
    }
}

Write-Host "Installed Knowledge Agent to $installRoot"
if ($NoPath) {
    Write-Host "PATH was not changed because -NoPath was provided."
}
elseif ($pathUpdated) {
    Write-Host "Added install directory to the current user's PATH."
    Write-Host "Open a new PowerShell window, then run: knowledge-agent --help"
}
else {
    Write-Host "Install directory is already in the current user's PATH."
}
Write-Host "Direct run: & `"$installRoot\knowledge-agent.exe`" --help"
