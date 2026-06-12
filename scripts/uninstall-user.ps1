param(
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

$installRoot = Get-SafeFullPath $InstallDir

if (Test-Path $installRoot) {
    Remove-Item -LiteralPath $installRoot -Recurse -Force
    Write-Host "Removed install directory: $installRoot"
}
else {
    Write-Host "Install directory does not exist: $installRoot"
}

if (-not $NoPath) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $pathEntries = @()
    if (-not [string]::IsNullOrWhiteSpace($userPath)) {
        $pathEntries = $userPath -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    }

    $keptEntries = @()
    $removedPath = $false
    foreach ($entry in $pathEntries) {
        if (Test-PathEntryEquals -Entry $entry -Target $installRoot) {
            $removedPath = $true
        }
        else {
            $keptEntries += $entry
        }
    }

    if ($removedPath) {
        [Environment]::SetEnvironmentVariable("Path", ($keptEntries -join ';'), "User")
        Write-Host "Removed install directory from the current user's PATH."
        Write-Host "Open a new PowerShell window for PATH changes to take effect."
    }
    else {
        Write-Host "Install directory was not found in the current user's PATH."
    }
}
else {
    Write-Host "PATH was not changed because -NoPath was provided."
}
