<#
.SYNOPSIS
    Syncs the version from VERSION file to all parser bindings.

.DESCRIPTION
    Reads D:\data\m3l\VERSION (or repo root VERSION) and updates:
    - parser/typescript/package.json          ("version": "x.y.z")
    - parser/typescript/src/resolver.ts       (PARSER_VERSION = 'x.y.z')
    - parser/csharp/src/M3L/M3L.csproj        (<Version>x.y.z</Version>)
    - parser/csharp/src/M3L/Resolver.cs       (ParserVersion = "x.y.z")

.PARAMETER DryRun
    Show what would change without modifying files.

.EXAMPLE
    .\update-version.ps1
    .\update-version.ps1 -DryRun
#>
param(
    [switch]$DryRun
)

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
    Write-Error "Invalid version format: '$version'. Expected semver (e.g. 0.1.0)"
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

# --- TypeScript ---
Write-Host "[TypeScript]" -ForegroundColor White

Update-File `
    -Path (Join-Path $root 'parser/typescript/package.json') `
    -Pattern '"version":\s*"[\d.]+"' `
    -Replacement "`"version`": `"$version`"" `
    -Label 'package.json version'

Update-File `
    -Path (Join-Path $root 'parser/typescript/src/resolver.ts') `
    -Pattern "PARSER_VERSION\s*=\s*'[\d.]+'" `
    -Replacement "PARSER_VERSION = '$version'" `
    -Label 'PARSER_VERSION'

Write-Host ""

# --- C# ---
Write-Host "[C#]" -ForegroundColor White

Update-File `
    -Path (Join-Path $root 'parser/csharp/src/M3L/M3L.csproj') `
    -Pattern '<Version>[\d.]+</Version>' `
    -Replacement "<Version>$version</Version>" `
    -Label 'csproj Version'

Update-File `
    -Path (Join-Path $root 'parser/csharp/src/M3L/Resolver.cs') `
    -Pattern 'ParserVersion\s*=\s*"[\d.]+"' `
    -Replacement "ParserVersion = `"$version`"" `
    -Label 'ParserVersion'

Write-Host ""

# --- Summary ---
if ($DryRun) {
    Write-Host "$updated file(s) would be updated." -ForegroundColor Yellow
} elseif ($updated -eq 0) {
    Write-Host "All files already at version $version." -ForegroundColor Green
} else {
    Write-Host "$updated file(s) updated to $version." -ForegroundColor Green
}
