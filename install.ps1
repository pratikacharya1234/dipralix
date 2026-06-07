# Dipralix installer for Windows (PowerShell)
#
# Downloads the prebuilt x86_64 Windows binary from GitHub Releases and
# installs it to %LOCALAPPDATA%\Dipralix\bin\, adding that directory to
# your user PATH so `dipralix-cli` works in any new terminal.
#
# Usage (PowerShell, no admin needed):
#   irm https://raw.githubusercontent.com/pratikacharya1234/dipralix/main/install.ps1 | iex
#
#   # or a specific version:
#   $env:DIPRALIX_VERSION="v0.1.0"
#   irm https://raw.githubusercontent.com/pratikacharya1234/dipralix/main/install.ps1 | iex

$ErrorActionPreference = 'Stop'

$Repo    = if ($env:DIPRALIX_REPO) { $env:DIPRALIX_REPO } else { 'Zyferon/dipralix' }
$BinName = 'dipralix-cli.exe'
$Version = if ($env:DIPRALIX_VERSION) { $env:DIPRALIX_VERSION } else { 'latest' }

# ── Arch detection ──────────────────────────────────────────────────────
$arch = [System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture
$AssetSuffix = switch ($arch) {
    'X64'   { 'windows-x86_64' }
    'Arm64' { 'windows-arm64' }
    default {
        Write-Host "No prebuilt Windows binary for architecture: $arch" -ForegroundColor Yellow
        Write-Host "Build from source instead:" -ForegroundColor Yellow
        Write-Host "  git clone https://github.com/$Repo"
        Write-Host "  cd dipralix"
        Write-Host "  cargo build --release"
        exit 1
    }
}

# ── Resolve version ─────────────────────────────────────────────────────
if ($Version -eq 'latest') {
    Write-Host "Resolving latest release tag…"
    $release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
    $Version = $release.tag_name
    if (-not $Version) {
        Write-Error "Could not resolve latest release tag. Set `$env:DIPRALIX_VERSION = 'v0.1.0' and retry."
        exit 1
    }
}

$Archive = "dipralix-$Version-$AssetSuffix.zip"
$Url     = "https://github.com/$Repo/releases/download/$Version/$Archive"

Write-Host "Installing Dipralix $Version for Windows x86_64…"
Write-Host "  → $Url"

# ── Download + extract ──────────────────────────────────────────────────
$tmp = Join-Path $env:TEMP ("dipralix-install-" + [System.Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $tmp | Out-Null
$archivePath = Join-Path $tmp $Archive

try {
    Invoke-WebRequest -Uri $Url -OutFile $archivePath -UseBasicParsing
    Expand-Archive -Path $archivePath -DestinationPath $tmp -Force

    $InstallDir = Join-Path $env:LOCALAPPDATA 'Dipralix\bin'
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

    $srcExe  = Join-Path $tmp $BinName
    $destExe = Join-Path $InstallDir $BinName

    if (-not (Test-Path $srcExe)) {
        Write-Error "Archive did not contain $BinName"
        exit 1
    }

    Copy-Item $srcExe $destExe -Force

    # ── Add to user PATH if not already there ────────────────────────────
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (-not ($userPath -split ';' | Where-Object { $_ -eq $InstallDir })) {
        Write-Host "Adding $InstallDir to user PATH…"
        $newPath = if ([string]::IsNullOrEmpty($userPath)) { $InstallDir } else { "$userPath;$InstallDir" }
        [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
        $env:Path = "$env:Path;$InstallDir"
        Write-Host "  (open a new terminal to pick up the PATH change)"
    }

    Write-Host ""
    Write-Host "Installed: $destExe" -ForegroundColor Green
    & $destExe --version
    Write-Host ""
    Write-Host "Run 'dipralix-cli' to start. Get a free API key at https://aistudio.google.com/apikey"
}
finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
