param(
    [string]$InstallRoot = "",
    [string]$CommandName = "search",
    [switch]$NoPath
)

$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$manifestPath = Join-Path $projectRoot "Cargo.toml"
$sourcePaperDir = [System.IO.Path]::GetFullPath((Join-Path $projectRoot "Paper"))

function Test-IsSameOrInside {
    param(
        [string]$Path,
        [string]$BasePath
    )

    $normalizedPath = [System.IO.Path]::GetFullPath($Path).TrimEnd("\")
    $normalizedBase = [System.IO.Path]::GetFullPath($BasePath).TrimEnd("\")
    return $normalizedPath -ieq $normalizedBase -or
        $normalizedPath.StartsWith("$normalizedBase\", [System.StringComparison]::OrdinalIgnoreCase)
}

if ($InstallRoot.Trim().Length -eq 0) {
    $InstallRoot = Join-Path $env:LOCALAPPDATA "topaperlist"
}

$InstallRoot = [System.IO.Path]::GetFullPath($InstallRoot)
$installRootTrimmed = $InstallRoot.TrimEnd("\")
$driveRoot = [System.IO.Path]::GetPathRoot($InstallRoot).TrimEnd("\")
if ($installRootTrimmed -ieq $driveRoot) {
    throw "InstallRoot must not be a drive root: $InstallRoot"
}

$binDir = Join-Path $InstallRoot "bin"
$paperDir = Join-Path $InstallRoot "Paper"
$installedBinary = Join-Path $binDir "$CommandName.exe"

if ((Test-IsSameOrInside -Path $paperDir -BasePath $sourcePaperDir) -or
    (Test-IsSameOrInside -Path $sourcePaperDir -BasePath $paperDir)) {
    throw "InstallRoot must not place installed Paper data inside, above, or equal to the source Paper directory."
}

if (-not (Test-Path -LiteralPath $sourcePaperDir)) {
    throw "Paper directory was not found at $sourcePaperDir"
}

$cargoCommand = (Get-Command cargo -ErrorAction SilentlyContinue).Source
if (-not $cargoCommand) {
    $cargoCandidate = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
    if (Test-Path -LiteralPath $cargoCandidate) {
        $cargoCommand = $cargoCandidate
    }
}

if (-not $cargoCommand) {
    throw "cargo was not found. Install Rust from https://rustup.rs/ and try again."
}
& $cargoCommand build --release --manifest-path $manifestPath

$builtBinary = Join-Path $projectRoot "target\release\search.exe"
if (-not (Test-Path -LiteralPath $builtBinary)) {
    throw "search.exe was not found at $builtBinary after building."
}

New-Item -ItemType Directory -Force -Path $binDir | Out-Null
Copy-Item -LiteralPath $builtBinary -Destination $installedBinary -Force

if (Test-Path -LiteralPath $paperDir) {
    Remove-Item -LiteralPath $paperDir -Recurse -Force
}
Copy-Item -LiteralPath $sourcePaperDir -Destination $paperDir -Recurse -Force

$expectedSmokeOutput = "B`tEMNLP`t2020`tAttention Is All You Need for Chinese Word Segmentation."
$smokeArgs = @(
    "--conference",
    "EMNLP",
    "--year",
    "2020",
    "attention",
    "is",
    "all",
    "you",
    "need"
)

Push-Location -LiteralPath $binDir
try {
    $smokeOutput = & $installedBinary @smokeArgs 2>&1
    $smokeStatus = $LASTEXITCODE
} finally {
    Pop-Location
}

$smokeLines = @(
    $smokeOutput |
        ForEach-Object { $_.ToString().Trim() } |
        Where-Object { $_.Length -gt 0 }
)

if ($smokeStatus -ne 0) {
    throw "Install smoke test failed with exit code ${smokeStatus}: $($smokeLines -join "`n")"
}

if ($smokeLines.Count -ne 1 -or $smokeLines[0] -ne $expectedSmokeOutput) {
    throw "Install smoke test output mismatch. Expected: $expectedSmokeOutput Actual: $($smokeLines -join "`n")"
}

if (-not $NoPath) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $pathItems = @()
    if ($userPath) {
        $pathItems = $userPath -split ";"
    }

    $alreadyInPath = $false
    foreach ($item in $pathItems) {
        if ($item.TrimEnd("\") -ieq $binDir.TrimEnd("\")) {
            $alreadyInPath = $true
            break
        }
    }

    if (-not $alreadyInPath) {
        if ($userPath -and $userPath.Trim().Length -gt 0) {
            $newPath = "$userPath;$binDir"
        } else {
            $newPath = $binDir
        }
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        $env:Path = "$env:Path;$binDir"
        Write-Host "Added $binDir to the user PATH. Open a new terminal if the command is not found."
    }
}

Write-Host "Installed $CommandName to $installedBinary"
Write-Host "Paper data installed to $paperDir"
Write-Host "Install smoke test passed"
Write-Host "Try: $CommandName --conference AAAI --year 2024 diffusion"
