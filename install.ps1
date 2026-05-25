<#
.SYNOPSIS
    Install / upgrade the topaperlist search tool from local source.
    Idempotent — safe to run repeatedly; re-builds and updates in place.
#>
param(
    [string]$InstallRoot = "",
    [string]$CommandName = "search",
    [string]$RepoUrl = "https://github.com/dududuguo/topaperlist.git",
    [string]$UpdateBranch = "main",
    [switch]$NoPath
)

$ErrorActionPreference = "Stop"
$OutputEncoding = [System.Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)

$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectDir = Join-Path $projectRoot "search"
$manifestPath = Join-Path $projectDir "Cargo.toml"
$sourcePapersDir = [System.IO.Path]::GetFullPath((Join-Path $projectRoot "PAPERS"))
$updateScriptSource = Join-Path $projectRoot "scripts\check-update.ps1"

# ── Safety helpers ─────────────────────────────────────────────

function Test-IsSameOrInside {
    param([string]$Path, [string]$BasePath)
    $normalizedPath = [System.IO.Path]::GetFullPath($Path).TrimEnd("\")
    $normalizedBase = [System.IO.Path]::GetFullPath($BasePath).TrimEnd("\")
    return $normalizedPath -ieq $normalizedBase -or
        $normalizedPath.StartsWith("$normalizedBase\", [System.StringComparison]::OrdinalIgnoreCase)
}

function Invoke-NativeCapture {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [string[]]$Arguments = @(),
        [string]$WorkingDirectory = ""
    )
    $oldEAP = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    if ($WorkingDirectory.Trim().Length -gt 0) { Push-Location -LiteralPath $WorkingDirectory }
    try {
        $output = & $FilePath @Arguments 2>&1
        $status = $LASTEXITCODE
    } finally {
        if ($WorkingDirectory.Trim().Length -gt 0) { Pop-Location }
        $ErrorActionPreference = $oldEAP
    }
    [PSCustomObject]@{
        ExitCode = $status
        Output   = @($output | ForEach-Object { $_.ToString() })
    }
}

# ── Resolve paths ──────────────────────────────────────────────

if ($InstallRoot.Trim().Length -eq 0) {
    $InstallRoot = Join-Path $env:LOCALAPPDATA "topaperlist"
}
$InstallRoot = [System.IO.Path]::GetFullPath($InstallRoot)
$installRootTrimmed = $InstallRoot.TrimEnd("\")
$driveRoot = [System.IO.Path]::GetPathRoot($InstallRoot).TrimEnd("\")
if ($installRootTrimmed -ieq $driveRoot) {
    throw "InstallRoot must not be a drive root: $InstallRoot"
}

$papersDir      = Join-Path $InstallRoot "PAPERS"
$dbPath         = Join-Path $InstallRoot "papers.db"
$installedBinary = Join-Path $InstallRoot "$CommandName-bin.exe"
$installedWrapper = Join-Path $InstallRoot "$CommandName.cmd"
$installedUpdateScript = Join-Path $InstallRoot "check-update.ps1"
$versionFile = Join-Path $InstallRoot "db.version"
$legacyCommandBinary = Join-Path $InstallRoot "$CommandName.exe"
$legacyDataDir  = Join-Path $InstallRoot "PaperJson"

if ((Test-IsSameOrInside -Path $papersDir -BasePath $sourcePapersDir) -or
    (Test-IsSameOrInside -Path $sourcePapersDir -BasePath $papersDir)) {
    throw "InstallRoot must not nest PAPERS inside the source PAPERS directory."
}

if (-not (Test-Path -LiteralPath $sourcePapersDir)) {
    throw "PAPERS directory was not found at $sourcePapersDir"
}
if (-not (Test-Path -LiteralPath $updateScriptSource)) {
    throw "Update script was not found at $updateScriptSource"
}
if (-not (Test-Path -LiteralPath $InstallRoot)) {
    New-Item -ItemType Directory -Path $InstallRoot | Out-Null
}

# ── Detect install mode ────────────────────────────────────────

$hasBinary  = Test-Path -LiteralPath $installedBinary
$hasWrapper = Test-Path -LiteralPath $installedWrapper
$hasData    = Test-Path -LiteralPath $papersDir
$hasDb      = Test-Path -LiteralPath $dbPath
$hasLegacy  = Test-Path -LiteralPath $legacyDataDir

Write-Host "Install root: $InstallRoot"
if ($hasLegacy) {
    Write-Host "Install mode: upgrade from legacy (PaperJson -> PAPERS + papers.db)"
} elseif ($hasBinary -or $hasWrapper -or $hasData -or $hasDb) {
    Write-Host "Install mode: upgrade existing install"
} else {
    Write-Host "Install mode: fresh install"
}

# ── Cargo detection (warn, do not auto-install) ────────────────

$cargoCommand = (Get-Command cargo -ErrorAction SilentlyContinue).Source
if (-not $cargoCommand) {
    $cargoCandidate = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
    if (Test-Path -LiteralPath $cargoCandidate) {
        $cargoCommand = $cargoCandidate
    }
}
if (-not $cargoCommand) {
    throw "cargo was not found on this system. Install Rust and cargo from https://rustup.rs/, then re-run this script."
}

$sourceVersion = "local"
$gitCommand = (Get-Command git -ErrorAction SilentlyContinue).Source
if ($gitCommand) {
    $revResult = Invoke-NativeCapture -FilePath $gitCommand -Arguments @("-C", $projectRoot, "rev-parse", "HEAD")
    if ($revResult.ExitCode -eq 0 -and $revResult.Output.Count -gt 0) {
        $sourceVersion = $revResult.Output[0].Trim()
    }
}

# ── Build ──────────────────────────────────────────────────────

Write-Host "Building $CommandName from source..."
& $cargoCommand build --release --manifest-path $manifestPath
if ($LASTEXITCODE -ne 0) {
    throw "Cargo build failed with exit code $LASTEXITCODE"
}

$builtBinary = Join-Path $projectDir "target\release\search.exe"
if (-not (Test-Path -LiteralPath $builtBinary)) {
    throw "search.exe was not found at $builtBinary after build."
}

# ── Install binary ─────────────────────────────────────────────

Copy-Item -LiteralPath $builtBinary -Destination $installedBinary -Force
Write-Host "Installed binary to $installedBinary"

Copy-Item -LiteralPath $updateScriptSource -Destination $installedUpdateScript -Force
Write-Host "Installed update script to $installedUpdateScript"

if ((Test-Path -LiteralPath $legacyCommandBinary) -and
    ([System.IO.Path]::GetFullPath($legacyCommandBinary) -ine [System.IO.Path]::GetFullPath($installedBinary))) {
    Remove-Item -LiteralPath $legacyCommandBinary -Force
    Write-Host "Removed legacy command binary at $legacyCommandBinary"
}

$wrapperContent = @"
@echo off
setlocal
set "TOPAPERLIST_INSTALL_ROOT=$InstallRoot"
set "TOPAPERLIST_BINARY=$installedBinary"
set "TOPAPERLIST_REPO_URL=$RepoUrl"
set "TOPAPERLIST_UPDATE_BRANCH=$UpdateBranch"
set "PAPERS_DIR=$papersDir"
set "PAPERS_DB_PATH=$dbPath"
if /I "%~1"=="update" (
  powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$installedUpdateScript" -InstallRoot "$InstallRoot" -RepoUrl "$RepoUrl" -Branch "$UpdateBranch" -Binary "$installedBinary" -Yes
  exit /b %ERRORLEVEL%
)
if not "%TOPAPERLIST_SKIP_UPDATE_CHECK%"=="1" (
  powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$installedUpdateScript" -InstallRoot "$InstallRoot" -RepoUrl "$RepoUrl" -Branch "$UpdateBranch" -Binary "$installedBinary" -Quiet
)
"$installedBinary" %*
exit /b %ERRORLEVEL%
"@
Set-Content -LiteralPath $installedWrapper -Value $wrapperContent -Encoding ASCII
Write-Host "Installed command wrapper to $installedWrapper"

# ── Install PAPERS data ────────────────────────────────────────

if (Test-Path -LiteralPath $papersDir) {
    Remove-Item -LiteralPath $papersDir -Recurse -Force
}
Copy-Item -LiteralPath $sourcePapersDir -Destination $papersDir -Recurse -Force
Write-Host "PAPERS data installed to $papersDir"

if (Test-Path -LiteralPath $legacyDataDir) {
    Remove-Item -LiteralPath $legacyDataDir -Recurse -Force
    Write-Host "Removed legacy PaperJson data at $legacyDataDir"
}

# ── Configure persistent environment variables ─────────────────

$env:PAPERS_DIR = $papersDir
$env:PAPERS_DB_PATH = $dbPath
$env:PAPERS_DB_VERSION = $sourceVersion
$env:PAPERS_DB_SOURCE = "$RepoUrl#$sourceVersion"
[Environment]::SetEnvironmentVariable("PAPERS_DIR", $papersDir, "User")
[Environment]::SetEnvironmentVariable("PAPERS_DB_PATH", $dbPath, "User")
Write-Host "Set user PAPERS_DIR=$papersDir"
Write-Host "Set user PAPERS_DB_PATH=$dbPath"

# ── Build database ─────────────────────────────────────────────

Write-Host "Building paper database..."
$buildDbResult = Invoke-NativeCapture -FilePath $installedBinary -Arguments @("build-db")
$buildDbResult.Output | ForEach-Object { Write-Host $_ }
if ($buildDbResult.ExitCode -ne 0) {
    throw "Database build failed with exit code $($buildDbResult.ExitCode)"
}
Set-Content -LiteralPath $versionFile -Value $sourceVersion -NoNewline
Write-Host "Recorded database version $sourceVersion"

# ── Smoke test ─────────────────────────────────────────────────

$smokeArgs = @("query", "--conference", "EMNLP", "--year", "2020",
                "attention", "is", "all", "you", "need")

$smokeResult = Invoke-NativeCapture -FilePath $installedBinary -Arguments $smokeArgs -WorkingDirectory $InstallRoot
$smokeStatus = $smokeResult.ExitCode
$smokeLines = @($smokeResult.Output |
    ForEach-Object { $_.ToString().Trim() } |
    Where-Object { $_.Length -gt 0 })

if ($smokeStatus -ne 0) {
    throw "Install smoke test failed with exit code ${smokeStatus}: $($smokeLines -join "`n")"
}

$matched = $smokeLines | Where-Object { $_ -match "Attention Is All You Need" }
if (-not $matched) {
    throw "Install smoke test output mismatch. Expected to contain: Attention Is All You Need for {C}hinese Word Segmentation. Actual: $($smokeLines -join "`n")"
}

# ── Add to PATH ────────────────────────────────────────────────

if (-not $NoPath) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $pathItems = @(if ($userPath) { $userPath -split ";" } else { @() })
    $alreadyInPath = $false
    foreach ($item in $pathItems) {
        if ($item.TrimEnd("\") -ieq $InstallRoot.TrimEnd("\")) {
            $alreadyInPath = $true
            break
        }
    }
    if (-not $alreadyInPath) {
        $newPath = if ($userPath -and $userPath.Trim().Length -gt 0) {
            "$userPath;$InstallRoot"
        } else {
            $InstallRoot
        }
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        $env:Path = "$env:Path;$InstallRoot"
        Write-Host "Added $InstallRoot to the user PATH."
    }
}

# ── Summary ────────────────────────────────────────────────────

Write-Host ""
Write-Host "topaperlist installed successfully."
Write-Host "  Command  : $CommandName"
Write-Host "  Wrapper  : $installedWrapper"
Write-Host "  Binary   : $installedBinary"
Write-Host "  Data     : $papersDir"
Write-Host "  Database : $dbPath"
Write-Host "  DB ver.  : $sourceVersion"
Write-Host ""

if (-not $NoPath -and -not $alreadyInPath) {
    Write-Host "Open a new terminal, then try: $CommandName query --conference AAAI --year 2024 diffusion"
} else {
    Write-Host "Try: $CommandName query --conference AAAI --year 2024 diffusion"
}
