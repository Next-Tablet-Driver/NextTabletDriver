<#
.SYNOPSIS
    Builds all NextTabletDriver dynamic plugins in release mode.
.DESCRIPTION
    Iterates over each plugin crate in the plugins/ directory, compiles it as a cdylib
    in release mode, and copies the resulting .dll (or .so) files to the target directory.
.PARAMETER OutDir
    Optional output folder where built plugin binaries will be copied.
    Defaults to %APPDATA%\NextTabletDriver\NextTabletReader\config\Settings\plugins
.PARAMETER Clean
    If specified, runs cargo clean on each plugin crate before building.
.PARAMETER Trust
    Adds the SHA-256 of every installed library to trusted_plugins.json (next to the plugins
    folder). NextTabletDriver only loads plugin libraries it has been told to trust, and each
    rebuild produces a new hash, so use this during development. Do not use it on libraries
    you did not build yourself.
#>
param (
    [string]$OutDir = "$env:APPDATA\NextTabletDriver\NextTabletReader\config\Settings\plugins",
    [switch]$Clean,
    [switch]$Trust
)

$ErrorActionPreference = "Continue"

$RootDir = Split-Path -Parent $PSScriptRoot
$PluginsDir = Join-Path $RootDir "plugins"

Write-Host "  NextTabletDriver - Dynamic Plugins Builder" -ForegroundColor Cyan

if (-not (Test-Path $PluginsDir)) {
    Write-Error "Plugins directory not found at $PluginsDir"
    exit 1
}

if (-not (Test-Path $OutDir)) {
    Write-Host "Creating output directory: $OutDir" -ForegroundColor Yellow
    New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
}

$PluginCrates = Get-ChildItem -Path $PluginsDir -Directory | Where-Object {
    Test-Path (Join-Path $_.FullName "Cargo.toml")
}

Write-Host "Discovered $($PluginCrates.Count) plugin crates:" -ForegroundColor Green
foreach ($crate in $PluginCrates) {
    Write-Host "  - $($crate.Name)" -ForegroundColor Gray
}
Write-Host ""

$SuccessCount = 0
$FailedCount = 0

$TrustFile = Join-Path (Split-Path -Parent $OutDir) "trusted_plugins.json"
function Add-TrustedHash([string]$Path) {
    $hash = (Get-FileHash -Path $Path -Algorithm SHA256).Hash.ToLower()
    $trusted = [ordered]@{}
    if (Test-Path $TrustFile) {
        $existing = Get-Content $TrustFile -Raw | ConvertFrom-Json
        if ($existing.trusted) {
            $existing.trusted.PSObject.Properties | ForEach-Object { $trusted[$_.Name] = $_.Value }
        }
    }
    $trusted[$hash] = Split-Path -Leaf $Path
    @{ trusted = $trusted } | ConvertTo-Json -Depth 4 | Set-Content -Path $TrustFile -Encoding utf8
}

foreach ($crate in $PluginCrates) {
    $CrateName = $crate.Name
    $CargoFile = Join-Path $crate.FullName "Cargo.toml"

    Write-Host "[$CrateName] Compiling in release mode..." -ForegroundColor Cyan -NoNewline

    if ($Clean) {
        cargo clean --manifest-path $CargoFile 2>$null
    }

    $buildOutput = cargo build --release --manifest-path $CargoFile 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host " [FAILED]" -ForegroundColor Red
        Write-Host $buildOutput -ForegroundColor DarkRed
        $FailedCount++
        continue
    }
    Write-Host " [OK]" -ForegroundColor Green

    # Locate the compiled cdylib
    $TargetRelease = Join-Path $crate.FullName "target\release"
    $DllFiles = Get-ChildItem -Path $TargetRelease -Filter "*.dll" -File | Where-Object {
        $_.Name -eq "$CrateName.dll"
    }

    if (-not $DllFiles) {
        # Fallback to any matching .so if on Linux / WSL
        $DllFiles = Get-ChildItem -Path $TargetRelease -Filter "*.so" -File | Where-Object {
            $_.Name -eq "lib$CrateName.so" -or $_.Name -eq "$CrateName.so"
        }
    }

    if ($DllFiles) {
        foreach ($file in $DllFiles) {
            $Dest = Join-Path $OutDir $file.Name
            Copy-Item -Path $file.FullName -Destination $Dest -Force
            Write-Host "  -> Installed: $($file.Name) to $OutDir" -ForegroundColor DarkGreen
            if ($Trust) {
                Add-TrustedHash $Dest
                Write-Host "  -> Trusted:   $($file.Name)" -ForegroundColor DarkGreen
            }
        }
        $SuccessCount++
    } else {
        Write-Host "  [!] Binary not found in $TargetRelease" -ForegroundColor Yellow
        $FailedCount++
    }
}

Write-Host ""
Write-Host "--------------------------------------------------" -ForegroundColor Cyan
Write-Host "Build Summary: $SuccessCount built successfully, $FailedCount failed." -ForegroundColor $(if ($FailedCount -eq 0) { "Green" } else { "Red" })
Write-Host "Plugins target folder: $OutDir" -ForegroundColor Gray
Write-Host "--------------------------------------------------" -ForegroundColor Cyan
