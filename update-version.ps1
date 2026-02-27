<#
.SYNOPSIS
    Syncs the version from VERSION file to all project manifests.

.DESCRIPTION
    Reads D:\data\m3l\VERSION (or repo root VERSION) and updates:
    - Cargo.toml (workspace)                  ([workspace.package] version = "x.y.z")
    - bindings/csharp/M3L.Native.csproj       (<Version>x.y.z</Version>)
    - bindings/typescript/package.json         ("version": "x.y.z")
    - bindings/typescript/package.json         ("@iyulab/m3l-napi": "x.y.z")
    - crates/m3l-napi/package.json            (version + optionalDependencies)
    - crates/m3l-napi/npm/*/package.json      (5 platform stub versions)
    - pkg/wasm/package.json                   ("version": "x.y.z")

.PARAMETER DryRun
    Show what would change without modifying files.

.EXAMPLE
    .\update-version.ps1
    .\update-version.ps1 -DryRun
    .\update-version.ps1 check
#>
param(
    [switch]$DryRun,
    [Parameter(Position=0)]
    [string]$Command
)

# Support positional "check" command as alias for -DryRun
if ($Command -eq 'check') {
    $DryRun = $true
}

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path

# Read version
$versionFile = Join-Path $root 'VERSION'
if (-not (Test-Path $versionFile)) {
    Write-Error "VERSION file not found at $versionFile"
    exit 1
}
$version = (Get-Content $versionFile -Raw).Trim()

if ($version -notmatch '^\d+\.\d+\.\d+$') {
    Write-Error "Invalid version format: '$version'. Expected semver (e.g. 0.4.0)"
    exit 1
}

Write-Host "Version: $version" -ForegroundColor Cyan
Write-Host ""

$updated = 0

function Update-File {
    param(
        [string]$Path,
        [string]$Pattern,
        [string]$Replacement,
        [string]$Label
    )

    $relativePath = $Path.Replace($root, '').TrimStart('\', '/')
    if (-not (Test-Path $Path)) {
        Write-Host "  SKIP  $relativePath (not found)" -ForegroundColor Yellow
        return
    }

    $content = Get-Content $Path -Raw -Encoding UTF8
    if ($content -match $Pattern) {
        $currentMatch = $Matches[0]
        $newContent = $content -replace $Pattern, $Replacement

        if ($content -eq $newContent) {
            Write-Host "  OK    $relativePath ($Label already $version)" -ForegroundColor DarkGray
            return
        }

        if ($DryRun) {
            Write-Host "  WOULD $relativePath : $currentMatch -> $Replacement" -ForegroundColor Yellow
        } else {
            # Preserve original encoding (UTF-8 no BOM)
            [System.IO.File]::WriteAllText($Path, $newContent, [System.Text.UTF8Encoding]::new($false))
            Write-Host "  SET   $relativePath ($Label -> $version)" -ForegroundColor Green
        }
        $script:updated++
    } else {
        Write-Host "  WARN  $relativePath (pattern not found: $Label)" -ForegroundColor Red
    }
}

# --- Rust workspace ---
Write-Host "[Rust workspace]" -ForegroundColor White

Update-File `
    -Path (Join-Path $root 'Cargo.toml') `
    -Pattern 'version\s*=\s*"[\d.]+"' `
    -Replacement "version = `"$version`"" `
    -Label 'workspace version'

Write-Host ""

# --- C# bindings ---
Write-Host "[C# bindings]" -ForegroundColor White

Update-File `
    -Path (Join-Path $root 'bindings/csharp/M3L.Native.csproj') `
    -Pattern '<Version>[\d.]+</Version>' `
    -Replacement "<Version>$version</Version>" `
    -Label 'csproj Version'

Write-Host ""

# --- TypeScript bindings ---
Write-Host "[TypeScript bindings]" -ForegroundColor White

Update-File `
    -Path (Join-Path $root 'bindings/typescript/package.json') `
    -Pattern '"version":\s*"[\d.]+"' `
    -Replacement "`"version`": `"$version`"" `
    -Label 'package.json version'

Update-File `
    -Path (Join-Path $root 'bindings/typescript/package.json') `
    -Pattern '"@iyulab/m3l-napi":\s*"[\d.]+"' `
    -Replacement "`"@iyulab/m3l-napi`": `"$version`"" `
    -Label 'napi dependency version'

Write-Host ""

# --- NAPI main package ---
Write-Host "[NAPI main package]" -ForegroundColor White

Update-File `
    -Path (Join-Path $root 'crates/m3l-napi/package.json') `
    -Pattern '"version":\s*"[\d.]+"' `
    -Replacement "`"version`": `"$version`"" `
    -Label 'napi version'

# Update each optionalDependency version in NAPI main package.json
$napiOptDeps = @(
    '@iyulab/m3l-napi-win32-x64-msvc',
    '@iyulab/m3l-napi-linux-x64-gnu',
    '@iyulab/m3l-napi-linux-x64-musl',
    '@iyulab/m3l-napi-darwin-x64',
    '@iyulab/m3l-napi-darwin-arm64'
)
foreach ($dep in $napiOptDeps) {
    $escapedDep = [regex]::Escape($dep)
    Update-File `
        -Path (Join-Path $root 'crates/m3l-napi/package.json') `
        -Pattern "`"$escapedDep`":\s*`"[\d.]+`"" `
        -Replacement "`"$dep`": `"$version`"" `
        -Label "$dep version"
}

Write-Host ""

# --- NAPI platform stubs ---
Write-Host "[NAPI platform stubs]" -ForegroundColor White

$platformDirs = @('win32-x64-msvc', 'linux-x64-gnu', 'linux-x64-musl', 'darwin-x64', 'darwin-arm64')
foreach ($dir in $platformDirs) {
    Update-File `
        -Path (Join-Path $root "crates/m3l-napi/npm/$dir/package.json") `
        -Pattern '"version":\s*"[\d.]+"' `
        -Replacement "`"version`": `"$version`"" `
        -Label "$dir version"
}

Write-Host ""

# --- WASM package ---
Write-Host "[WASM package]" -ForegroundColor White

Update-File `
    -Path (Join-Path $root 'pkg/wasm/package.json') `
    -Pattern '"version":\s*"[\d.]+"' `
    -Replacement "`"version`": `"$version`"" `
    -Label 'wasm version'

Write-Host ""

# --- Summary ---
if ($DryRun) {
    Write-Host "$updated file(s) would be updated." -ForegroundColor Yellow
} elseif ($updated -eq 0) {
    Write-Host "All files already at version $version." -ForegroundColor Green
} else {
    Write-Host "$updated file(s) updated to $version." -ForegroundColor Green
}
