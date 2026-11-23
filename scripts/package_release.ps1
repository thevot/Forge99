# ==============================================================================
# Forge99 — 64-bit Level Authoring Studio for Unreal Tournament 99
# Standalone Release Packager
# ==============================================================================
param (
    [string]$OutputDir = "dist/forge99-release",
    [switch]$SkipBuild,
    [switch]$Zip
)

$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " Forge99 - Standalone Release Packaging (x64)              " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

$WorkspaceRoot = (Get-Item $PSScriptRoot).Parent.FullName
Set-Location $WorkspaceRoot

$TargetDir = Join-Path $WorkspaceRoot $OutputDir
Write-Host "[1/5] Preparing output directory: $TargetDir" -ForegroundColor Yellow

if (Test-Path -LiteralPath $TargetDir) {
    throw "Release output already exists: $TargetDir. Choose a new output directory to preserve existing files."
}
New-Item -ItemType Directory -Path $TargetDir -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $TargetDir "Textures") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $TargetDir "Maps") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $TargetDir "Palettes") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $TargetDir "Presets") -Force | Out-Null

# 2. Compile Release Binary
if (-not $SkipBuild) {
    Write-Host "[2/5] Compiling optimized release binary (cargo build --release --bin forge99)..." -ForegroundColor Yellow
    & cargo build --release --bin forge99
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Cargo release build failed with exit code $LASTEXITCODE"
        exit $LASTEXITCODE
    }
} else {
    Write-Host "[2/5] Skipping cargo build step (-SkipBuild specified)" -ForegroundColor DarkGray
}

$BinarySource = Join-Path $WorkspaceRoot "target/release/forge99.exe"
if (-not (Test-Path $BinarySource)) {
    throw "Release executable not found at $BinarySource. Build the release target before packaging."
}

$BinaryDest = Join-Path $TargetDir "forge99.exe"
Copy-Item $BinarySource $BinaryDest -Force
$BinarySizeMb = [math]::Round(((Get-Item $BinaryDest).Length / 1MB), 2)
Write-Host "  -> Bundled forge99.exe ($BinarySizeMb MB)" -ForegroundColor Green

# 3. Copy License & Documentation (No proprietary UT99 assets bundled)
Write-Host "[3/5] Bundling open-source documentation and license..." -ForegroundColor Yellow
Copy-Item (Join-Path $WorkspaceRoot "README.md") $TargetDir -Force -ErrorAction SilentlyContinue
Copy-Item (Join-Path $WorkspaceRoot "LICENSE-MIT") $TargetDir -Force -ErrorAction SilentlyContinue
Copy-Item (Join-Path $WorkspaceRoot "LICENSE-APACHE") $TargetDir -Force -ErrorAction SilentlyContinue
Copy-Item (Join-Path $WorkspaceRoot "CHANGELOG.md") $TargetDir -Force -ErrorAction SilentlyContinue

# Extract version from root Cargo.toml
$CargoTomlPath = Join-Path $WorkspaceRoot "Cargo.toml"
$Version = "0.13.0"
if (Test-Path $CargoTomlPath) {
    $CargoTomlContent = Get-Content $CargoTomlPath -Raw
    if ($CargoTomlContent -match '(?m)^version\s*=\s*"([^"]+)"') {
        $Version = $Matches[1]
    }
}
Write-Host "Detected package version: v$Version" -ForegroundColor Cyan
$ReleaseNotes = Join-Path $WorkspaceRoot "docs/RELEASE_NOTES_$Version.md"
if (Test-Path -LiteralPath $ReleaseNotes) {
    $DocsDir = Join-Path $TargetDir "docs"
    New-Item -ItemType Directory -Path $DocsDir -Force | Out-Null
    Copy-Item -LiteralPath $ReleaseNotes -Destination $DocsDir
} elseif ($Version -match '-') {
    throw "Preview release notes not found: $ReleaseNotes"
}

# 4. Generate Comprehensive Standalone Documentation
Write-Host "[4/5] Generating standalone README and Quick Reference..." -ForegroundColor Yellow

$ReadmeContent = @"
Forge99 $Version — Windows x64 preview

This is an intermediate preview for evaluating the current Map Wizard work.
Run forge99.exe and configure your own Unreal Tournament installation. No game
assets are bundled. See README.md for general usage and the preview release
notes for changes, test coverage and known limitations.

Important: canonical corridors may disconnect when rooms are moved or resized.
The wizard reports some risks, but static checks do not prove a map is playable.
Test generated maps in an isolated UT99 installation before relying on them.
"@

$ReadmePath = Join-Path $TargetDir "README.txt"
Set-Content -Path $ReadmePath -Value $ReadmeContent -Encoding UTF8
Write-Host "  -> Generated $ReadmePath" -ForegroundColor Green

# 5. Optional ZIP Archiving
if ($Zip) {
    Write-Host "[5/5] Creating release distribution ZIP archive..." -ForegroundColor Yellow
    $DistDir = Split-Path $TargetDir -Parent
    $ZipPath = Join-Path $DistDir "forge99-v$Version-windows-x64.zip"
    if (Test-Path -LiteralPath $ZipPath) {
        throw "Release archive already exists: $ZipPath. Move or rename it before packaging."
    }
    Compress-Archive -Path "$TargetDir\*" -DestinationPath $ZipPath -CompressionLevel Optimal
    $ZipSizeMb = [math]::Round(((Get-Item $ZipPath).Length / 1MB), 2)
    Write-Host "  -> Successfully packaged ZIP: $ZipPath ($ZipSizeMb MB)" -ForegroundColor Cyan
} else {
    Write-Host "[5/5] Standalone distribution ready at: $TargetDir" -ForegroundColor Cyan
}

Write-Host "==========================================================" -ForegroundColor Green
Write-Host " Packaging Completed Successfully!                        " -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green
