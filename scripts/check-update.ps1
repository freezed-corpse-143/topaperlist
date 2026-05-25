<#
.SYNOPSIS
    Check whether installed topaperlist data is behind the configured Git branch.

.DESCRIPTION
    The database version is the Git commit SHA of the data branch used to build
    papers.db. This script compares the local version with the remote branch,
    prompts the user when an update is available, then fetches the managed local
    repo, copies PAPERS, and rebuilds papers.db.
#>
param(
    [string]$InstallRoot = "",
    [string]$RepoUrl = "",
    [string]$Branch = "",
    [string]$Binary = "",
    [switch]$Yes,
    [switch]$SkipThisVersion,
    [switch]$Quiet
)

$ErrorActionPreference = "Stop"
$explicitInstallRoot = $PSBoundParameters.ContainsKey("InstallRoot") -and $InstallRoot.Trim().Length -gt 0

function Write-Info {
    param([string]$Message)
    if (-not $Quiet) {
        Write-Host $Message
    }
}

function Invoke-Native {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [string[]]$Arguments = @(),
        [string]$WorkingDirectory = ""
    )

    $oldEAP = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    if ($WorkingDirectory.Trim().Length -gt 0) {
        Push-Location -LiteralPath $WorkingDirectory
    }
    try {
        $output = & $FilePath @Arguments 2>&1
        $status = $LASTEXITCODE
    } finally {
        if ($WorkingDirectory.Trim().Length -gt 0) {
            Pop-Location
        }
        $ErrorActionPreference = $oldEAP
    }

    [PSCustomObject]@{
        ExitCode = $status
        Output = @($output | ForEach-Object { $_.ToString() })
    }
}

function Short-Sha {
    param([string]$Sha)
    if ($Sha.Length -ge 12) {
        return $Sha.Substring(0, 12)
    }
    if ($Sha.Trim().Length -eq 0) {
        return "none"
    }
    return $Sha
}

function Ensure-InstallRoot {
    if (-not (Test-Path -LiteralPath $InstallRoot)) {
        New-Item -ItemType Directory -Path $InstallRoot | Out-Null
    }
}

if ($RepoUrl.Trim().Length -eq 0) {
    $RepoUrl = if ($env:TOPAPERLIST_REPO_URL) {
        $env:TOPAPERLIST_REPO_URL
    } else {
        "https://github.com/dududuguo/topaperlist.git"
    }
}

if ($Branch.Trim().Length -eq 0) {
    $Branch = if ($env:TOPAPERLIST_UPDATE_BRANCH) {
        $env:TOPAPERLIST_UPDATE_BRANCH
    } else {
        "main"
    }
}

if ($InstallRoot.Trim().Length -eq 0) {
    $InstallRoot = if ($env:TOPAPERLIST_INSTALL_ROOT) {
        $env:TOPAPERLIST_INSTALL_ROOT
    } elseif ($env:LOCALAPPDATA) {
        Join-Path $env:LOCALAPPDATA "topaperlist"
    } else {
        Split-Path -Parent $MyInvocation.MyCommand.Path
    }
}
$InstallRoot = [System.IO.Path]::GetFullPath($InstallRoot)

$papersDir = if ((-not $explicitInstallRoot) -and $env:PAPERS_DIR) {
    $env:PAPERS_DIR
} else {
    Join-Path $InstallRoot "PAPERS"
}
$dbPath = if ((-not $explicitInstallRoot) -and $env:PAPERS_DB_PATH) {
    $env:PAPERS_DB_PATH
} else {
    Join-Path $InstallRoot "papers.db"
}
$repoDir = if ($env:TOPAPERLIST_REPO_DIR) { $env:TOPAPERLIST_REPO_DIR } else { Join-Path $InstallRoot "repo" }
$versionFile = Join-Path $InstallRoot "db.version"
$skippedVersionFile = Join-Path $InstallRoot "skipped.version"
$managedMarker = Join-Path $repoDir ".topaperlist-managed"

if ($Binary.Trim().Length -eq 0) {
    $Binary = if ($env:TOPAPERLIST_BINARY) {
        $env:TOPAPERLIST_BINARY
    } else {
        Join-Path $InstallRoot "search-bin.exe"
    }
}

$gitCommand = (Get-Command git -ErrorAction SilentlyContinue).Source
if (-not $gitCommand) {
    Write-Info "topaperlist update check skipped: git was not found."
    exit 0
}

$remoteRef = "refs/heads/$Branch"
$remoteResult = Invoke-Native -FilePath $gitCommand -Arguments @("ls-remote", $RepoUrl, $remoteRef)
if ($remoteResult.ExitCode -ne 0 -or $remoteResult.Output.Count -eq 0) {
    Write-Info "topaperlist update check skipped: unable to reach $RepoUrl."
    exit 0
}

$remoteVersion = ($remoteResult.Output[0] -split "\s+")[0]
if ($remoteVersion.Trim().Length -eq 0) {
    Write-Info "topaperlist update check skipped: remote version was empty."
    exit 0
}

$localVersion = ""
if (Test-Path -LiteralPath $versionFile) {
    $localVersion = (Get-Content -LiteralPath $versionFile -Raw).Trim()
} elseif (Test-Path -LiteralPath (Join-Path $repoDir ".git")) {
    $revResult = Invoke-Native -FilePath $gitCommand -Arguments @("-C", $repoDir, "rev-parse", "HEAD")
    if ($revResult.ExitCode -eq 0 -and $revResult.Output.Count -gt 0) {
        $localVersion = $revResult.Output[0].Trim()
    }
}

$localShort = Short-Sha $localVersion
$remoteShort = Short-Sha $remoteVersion

if ($localVersion -eq $remoteVersion) {
    Write-Info "topaperlist data already up to date ($remoteShort)."
    exit 0
}

if ($SkipThisVersion) {
    Ensure-InstallRoot
    Set-Content -LiteralPath $skippedVersionFile -Value $remoteVersion -NoNewline
    Write-Info "Skipped topaperlist data version $(Short-Sha $remoteVersion)."
    exit 0
}

if ((-not $Yes) -and (Test-Path -LiteralPath $skippedVersionFile)) {
    $skippedVersion = (Get-Content -LiteralPath $skippedVersionFile -Raw).Trim()
    if ($skippedVersion -eq $remoteVersion) {
        exit 0
    }
}

if (-not $Yes) {
    if (-not [Environment]::UserInteractive) {
        exit 0
    }

    Write-Host "topaperlist data update available: $localShort -> $remoteShort"
    $updateChosen = $false
    while (-not $updateChosen) {
        $answer = Read-Host "Choose: [u]pdate / [s]kip this version / [c]ancel"
        switch -Regex ($answer.Trim()) {
            "^(u|update|y|yes)$" {
                $Quiet = $false
                $updateChosen = $true
            }
            "^(s|skip|skip this version)$" {
                Ensure-InstallRoot
                Set-Content -LiteralPath $skippedVersionFile -Value $remoteVersion -NoNewline
                $Quiet = $false
                Write-Info "Skipped topaperlist data version $(Short-Sha $remoteVersion)."
                exit 0
            }
            "^(c|cancel|n|no)?$" {
                exit 0
            }
            default {
                Write-Host "Please choose u, s, or c."
            }
        }
    }
} else {
    Write-Info "topaperlist data update available: $localShort -> $remoteShort"
}

Ensure-InstallRoot

if (Test-Path -LiteralPath (Join-Path $repoDir ".git")) {
    if (-not (Test-Path -LiteralPath $managedMarker)) {
        throw "Refusing to update unmanaged repo at $repoDir. Remove it or set TOPAPERLIST_REPO_DIR to a managed directory."
    }
    Write-Info "Fetching topaperlist data from $RepoUrl..."
    $fetch = Invoke-Native -FilePath $gitCommand -Arguments @("-C", $repoDir, "fetch", "--depth=1", "origin", $Branch)
    if ($fetch.ExitCode -ne 0) {
        throw "git fetch failed: $($fetch.Output -join "`n")"
    }
    $checkout = Invoke-Native -FilePath $gitCommand -Arguments @("-C", $repoDir, "checkout", "-B", $Branch, "FETCH_HEAD")
    if ($checkout.ExitCode -ne 0) {
        throw "git checkout failed: $($checkout.Output -join "`n")"
    }
} else {
    if (Test-Path -LiteralPath $repoDir) {
        $children = @(Get-ChildItem -Force -LiteralPath $repoDir)
        if ($children.Count -gt 0) {
            throw "Refusing to clone into non-empty directory: $repoDir"
        }
    } else {
        New-Item -ItemType Directory -Path (Split-Path -Parent $repoDir) -Force | Out-Null
    }

    Write-Info "Cloning topaperlist data from $RepoUrl..."
    $clone = Invoke-Native -FilePath $gitCommand -Arguments @("clone", "--depth=1", "--branch", $Branch, $RepoUrl, $repoDir)
    if ($clone.ExitCode -ne 0) {
        throw "git clone failed: $($clone.Output -join "`n")"
    }
    New-Item -ItemType File -Path $managedMarker -Force | Out-Null
}

$sourcePapers = Join-Path $repoDir "PAPERS"
if (-not (Test-Path -LiteralPath $sourcePapers)) {
    throw "PAPERS directory was not found in updated repo: $sourcePapers"
}

if (Test-Path -LiteralPath $papersDir) {
    Remove-Item -LiteralPath $papersDir -Recurse -Force
}
Copy-Item -LiteralPath $sourcePapers -Destination $papersDir -Recurse -Force

if (-not (Test-Path -LiteralPath $Binary)) {
    throw "Search binary was not found: $Binary"
}

$env:PAPERS_DIR = $papersDir
$env:PAPERS_DB_PATH = $dbPath
$env:PAPERS_DB_VERSION = $remoteVersion
$env:PAPERS_DB_SOURCE = "$RepoUrl#$remoteVersion"

Write-Info "Rebuilding paper database..."
$build = Invoke-Native -FilePath $Binary -Arguments @("build-db") -WorkingDirectory $InstallRoot
if ($build.ExitCode -ne 0) {
    throw "Database rebuild failed: $($build.Output -join "`n")"
}

Set-Content -LiteralPath $versionFile -Value $remoteVersion -NoNewline
if (Test-Path -LiteralPath $skippedVersionFile) {
    Remove-Item -LiteralPath $skippedVersionFile -Force
}
Write-Info "topaperlist data updated to $(Short-Sha $remoteVersion)."
