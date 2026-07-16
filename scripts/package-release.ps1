[CmdletBinding()]
param(
    [string]$OutputDirectory = "release",
    [switch]$SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$cargoToml = Join-Path $repoRoot "Cargo.toml"
$versionMatch = Select-String -Path $cargoToml -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
if (-not $versionMatch) {
    throw "Unable to read the workspace version from Cargo.toml"
}

$version = $versionMatch.Matches[0].Groups[1].Value
$packageName = "FPW-v$version"
$releaseRoot = if ([IO.Path]::IsPathRooted($OutputDirectory)) {
    [IO.Path]::GetFullPath($OutputDirectory)
} else {
    [IO.Path]::GetFullPath((Join-Path $repoRoot $OutputDirectory))
}
$packageDirectory = Join-Path $releaseRoot $packageName
$archivePath = Join-Path $releaseRoot "$packageName.zip"
$packageTarget = Join-Path $repoRoot "target\package-release"

function Invoke-CheckedCommand {
    param(
        [Parameter(Mandatory = $true)]
        [scriptblock]$Command,
        [Parameter(Mandatory = $true)]
        [string]$Description
    )

    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "$Description failed with exit code $LASTEXITCODE"
    }
}

function Remove-PackageArtifact {
    param([Parameter(Mandatory = $true)][string]$Path)

    if (-not (Test-Path -LiteralPath $Path)) {
        return
    }

    $resolvedParent = [IO.Path]::GetFullPath((Split-Path $Path -Parent))
    if (-not $resolvedParent.Equals($releaseRoot, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to remove an artifact outside the release directory: $Path"
    }
    Remove-Item -LiteralPath $Path -Recurse -Force
}

function New-ReleaseArchive {
    param(
        [Parameter(Mandatory = $true)][string]$Source,
        [Parameter(Mandatory = $true)][string]$Destination
    )

    if (-not (Get-Command tar.exe -ErrorAction SilentlyContinue)) {
        throw "tar.exe is required to create the Windows ZIP release"
    }
    if (Test-Path -LiteralPath $Destination) {
        Remove-Item -LiteralPath $Destination -Force
    }

    $sourceParent = Split-Path $Source -Parent
    $sourceName = Split-Path $Source -Leaf
    Push-Location $sourceParent
    try {
        & tar.exe -a -c -f $Destination $sourceName
        if ($LASTEXITCODE -ne 0) {
            throw "tar.exe failed with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }
}

Push-Location $repoRoot
try {
    if (-not $SkipBuild) {
        Push-Location (Join-Path $repoRoot "web")
        try {
            Invoke-CheckedCommand -Description "WebUI build" -Command { npm run build }
        } finally {
            Pop-Location
        }

        Invoke-CheckedCommand -Description "FPW release build" -Command {
            cargo build --release -p fpw-cli --target-dir $packageTarget
        }
        $sourceExe = Join-Path $packageTarget "release\fpw.exe"
    } else {
        $sourceExe = Join-Path $repoRoot "target\release\fpw.exe"
    }

    $requiredPaths = @(
        $sourceExe,
        (Join-Path $repoRoot "web\dist\index.html"),
        (Join-Path $repoRoot "web\dist\assets"),
        (Join-Path $repoRoot "README-CN.md"),
        (Join-Path $repoRoot "User-Manual-CN.md")
    )
    foreach ($requiredPath in $requiredPaths) {
        if (-not (Test-Path -LiteralPath $requiredPath)) {
            throw "Required release file is missing: $requiredPath"
        }
    }

    New-Item -ItemType Directory -Force -Path $releaseRoot | Out-Null
    Remove-PackageArtifact -Path $packageDirectory
    Remove-PackageArtifact -Path $archivePath

    New-Item -ItemType Directory -Force -Path $packageDirectory | Out-Null
    Copy-Item -LiteralPath $sourceExe -Destination (Join-Path $packageDirectory "fpw.exe")

    $webDirectory = Join-Path $packageDirectory "web"
    New-Item -ItemType Directory -Force -Path $webDirectory | Out-Null
    Copy-Item -LiteralPath (Join-Path $repoRoot "web\dist") -Destination (Join-Path $webDirectory "dist") -Recurse

    $workflowDestination = Join-Path $packageDirectory "workflows"
    New-Item -ItemType Directory -Force -Path $workflowDestination | Out-Null
    $workflowSource = Join-Path $repoRoot "workflows"
    if (Test-Path -LiteralPath $workflowSource) {
        Get-ChildItem -LiteralPath $workflowSource -Force |
            Where-Object { $_.Name -ne ".trash" } |
            Copy-Item -Destination $workflowDestination -Recurse
    }

    $examplesSource = Join-Path $repoRoot "examples"
    if (Test-Path -LiteralPath $examplesSource) {
        Copy-Item -LiteralPath $examplesSource -Destination (Join-Path $packageDirectory "examples") -Recurse
    }

    Copy-Item -LiteralPath (Join-Path $repoRoot "README-CN.md") -Destination $packageDirectory
    Copy-Item -LiteralPath (Join-Path $repoRoot "User-Manual-CN.md") -Destination $packageDirectory

    $packagedExe = Join-Path $packageDirectory "fpw.exe"
    $reportedVersion = (& $packagedExe --version).Trim()
    if ($reportedVersion -ne "fpw $version") {
        throw "Packaged executable version mismatch: expected 'fpw $version', got '$reportedVersion'"
    }

    New-ReleaseArchive -Source $packageDirectory -Destination $archivePath
    $archive = Get-Item -LiteralPath $archivePath
    $hash = Get-FileHash -LiteralPath $archivePath -Algorithm SHA256

    [pscustomobject]@{
        Version = "v$version"
        PackageDirectory = $packageDirectory
        Archive = $archive.FullName
        ArchiveBytes = $archive.Length
        SHA256 = $hash.Hash
    } | Format-List
} finally {
    Pop-Location
}
