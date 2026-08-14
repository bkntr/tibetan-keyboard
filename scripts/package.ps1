[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('x86_64-pc-windows-msvc', 'aarch64-pc-windows-msvc')]
    [string]$Target,

    [Parameter(Mandatory = $true)]
    [ValidateSet('x64', 'arm64')]
    [string]$Architecture,

    [string]$Version,

    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot

$expectedArchitecture = if ($Target.StartsWith('x86_64-')) { 'x64' } else { 'arm64' }
if ($Architecture -ne $expectedArchitecture) {
    throw "Target $Target must use installer architecture $expectedArchitecture."
}

if (-not $Version) {
    $metadata = cargo metadata --no-deps --format-version 1 |
        ConvertFrom-Json
    $package = $metadata.packages |
        Where-Object { $_.name -eq 'tibetan-ewts-keyboard' } |
        Select-Object -First 1
    if (-not $package) {
        throw 'Unable to read the application version from Cargo metadata.'
    }
    $Version = $package.version
}

if ($Version -notmatch '^\d+\.\d+\.\d+([+-][0-9A-Za-z.-]+)?$') {
    throw "Version '$Version' is not a valid release version."
}

if (-not $SkipBuild) {
    rustup target add $Target
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to install Rust target $Target."
    }
    cargo build --release --locked --target $Target
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo build failed for $Target."
    }
}

$builtExe = Join-Path $workspace "target\$Target\release\tibetan-ewts-keyboard.exe"
if (-not (Test-Path -LiteralPath $builtExe -PathType Leaf)) {
    throw "Built executable not found at $builtExe."
}

$distDir = Join-Path $workspace 'dist'
New-Item -ItemType Directory -Path $distDir -Force | Out-Null

$standaloneName = "tibetan-ewts-keyboard-$Version-$Target-standalone.exe"
$standalonePath = Join-Path $distDir $standaloneName
Copy-Item -LiteralPath $builtExe -Destination $standalonePath -Force

$isccCommand = Get-Command 'ISCC.exe' -ErrorAction SilentlyContinue
if ($isccCommand) {
    $isccPath = $isccCommand.Source
} else {
    $isccPath = Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'
}
if (-not (Test-Path -LiteralPath $isccPath -PathType Leaf)) {
    throw 'Inno Setup 6 was not found. Install it or add ISCC.exe to PATH.'
}

$installerName = "tibetan-ewts-keyboard-$Version-windows-$Architecture-setup"
$installerScript = Join-Path $workspace 'installer\tibetan-ewts-keyboard.iss'
$env:TIBETAN_EWTS_APP_VERSION = $Version
$env:TIBETAN_EWTS_APP_ARCH = $Architecture
$env:TIBETAN_EWTS_SOURCE_EXE = $builtExe
$env:TIBETAN_EWTS_OUTPUT_DIR = $distDir
$env:TIBETAN_EWTS_OUTPUT_NAME = $installerName
$env:TIBETAN_EWTS_ICON_FILE = Join-Path $workspace 'assets\tibetan-ewts-keyboard.ico'
& $isccPath $installerScript
if ($LASTEXITCODE -ne 0) {
    throw "Inno Setup failed for $Architecture."
}

$installerPath = Join-Path $distDir "$installerName.exe"
if (-not (Test-Path -LiteralPath $installerPath -PathType Leaf)) {
    throw "Installer not found at $installerPath."
}

Write-Output "Packaged $standalonePath"
Write-Output "Packaged $installerPath"
