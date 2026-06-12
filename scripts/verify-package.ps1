param(
    [string]$PackageDir = "dist\knowledge-agent",
    [int]$Port = 3047
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$packageRoot = Resolve-Path (Join-Path $repoRoot $PackageDir)
$exe = Join-Path $packageRoot "knowledge-agent.exe"
$tempVault = Join-Path ([System.IO.Path]::GetTempPath()) ("knowledge-agent-vault-" + [guid]::NewGuid())
$process = $null

if (-not (Test-Path $exe)) {
    throw "Missing packaged executable: $exe"
}

try {
    New-Item -ItemType Directory -Path $tempVault | Out-Null

    & $exe init $tempVault | Out-Host
    if (-not (Test-Path (Join-Path $tempVault ".knowledge-agent.toml"))) {
        throw "init did not create .knowledge-agent.toml"
    }
    if (-not (Test-Path (Join-Path $tempVault ".knowledge-agent"))) {
        throw "init did not create .knowledge-agent"
    }

    $process = Start-Process `
        -FilePath $exe `
        -ArgumentList @("serve", $tempVault, "--port", $Port) `
        -WorkingDirectory $packageRoot `
        -WindowStyle Hidden `
        -PassThru

    $ready = $false
    for ($i = 0; $i -lt 40; $i++) {
        try {
            $health = Invoke-WebRequest -UseBasicParsing "http://127.0.0.1:$Port/api/health" -TimeoutSec 2
            if ($health.StatusCode -eq 200) {
                $ready = $true
                break
            }
        }
        catch {
            Start-Sleep -Milliseconds 250
        }
    }

    if (-not $ready) {
        throw "packaged server did not become ready"
    }

    $page = Invoke-WebRequest -UseBasicParsing "http://127.0.0.1:$Port/" -TimeoutSec 5
    if ($page.Content -notmatch '<div id="root">') {
        throw "web root did not return built index.html"
    }

    Write-Host "Package verification passed"
}
finally {
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force
    }
    if (Test-Path $tempVault) {
        Remove-Item -LiteralPath $tempVault -Recurse -Force
    }
}
