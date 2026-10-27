[CmdletBinding()]
param(
    [string]$Horizon,
    [switch]$Prepare,
    [string]$MinecraftDir,
    [string]$ServerJar,
    [int]$Jobs = 2
)
$ErrorActionPreference = 'Stop'
$mods = Split-Path -Parent $PSScriptRoot
if (-not $Horizon) { $Horizon = Join-Path (Split-Path -Parent $mods) 'horizon' }
$project = Join-Path $mods "Tinkers' onstruct"
# Horizon's launcher links the user's Prism files in this official layout.
if (-not $MinecraftDir) {
    $linked = Join-Path $env:APPDATA 'horizon-launcher\linked-minecraft'
    if (Test-Path -LiteralPath (Join-Path $linked 'versions\26.3\26.3.json') -PathType Leaf) {
        $MinecraftDir = $linked
    } else {
        $MinecraftDir = Join-Path $env:APPDATA '.minecraft'
    }
}
if (-not $ServerJar) {
    $existingServer = Join-Path $env:APPDATA 'horizon-launcher\server-jars\26.3.jar'
    if (Test-Path -LiteralPath $existingServer -PathType Leaf) { $ServerJar = $existingServer }
}
if (-not (Test-Path -LiteralPath (Join-Path $Horizon 'research-lock.json') -PathType Leaf)) { throw "Horizon checkout not found: $Horizon. Clone it beside horizon-mods or pass -Horizon." }
if ($Jobs -lt 1) { throw 'Jobs must be at least 1.' }
foreach ($name in @('cargo', 'rustup')) { if (-not (Get-Command $name -ErrorAction SilentlyContinue)) { throw "$name not found. Install Rust stable with the MSVC toolchain first." } }
function Invoke-Checked([string]$Exe, [string[]]$Arguments) {
    & $Exe @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Exe failed with exit code $LASTEXITCODE" }
}
Invoke-Checked 'rustup' @('target', 'add', 'wasm32-unknown-unknown')
Push-Location $Horizon
try {
    Invoke-Checked 'cargo' @('build', '--locked', '--profile', 'fast', '-j', "$Jobs", '-p', 'horizon-client', '--features', 'mods', '--bin', 'horizon-client', '--bin', 'horizon-ctl')
    Invoke-Checked 'cargo' @('build', '--locked', '-j', "$Jobs", '-p', 'cargo-horizon-mod')
    if ($Prepare) {
        if (-not $ServerJar -or -not (Test-Path -LiteralPath $ServerJar -PathType Leaf)) { throw 'Pass -ServerJar with your official Minecraft 26.3 server jar.' }
        if (-not (Get-Command 'javac' -ErrorAction SilentlyContinue)) { throw 'JDK 25 (java and javac) is required to prepare resources.' }
        Invoke-Checked 'cargo' @('run', '--locked', '--release', '-j', "$Jobs", '-p', 'horizon-prepare', '--', '--prepare', '--root', $Horizon, '--minecraft-dir', $MinecraftDir, '--server-jar', $ServerJar)
    }
} finally { Pop-Location }
$builder = Join-Path $Horizon 'target\debug\cargo-horizon-mod.exe'
foreach ($dir in @($project, (Join-Path $project 'server-tools'), (Join-Path $project 'probes\server-lifecycle'))) {
    Push-Location $dir
    try { Invoke-Checked $builder @('horizon-mod', 'build') } finally { Pop-Location }
}
$resources = Join-Path $Horizon '.local\horizon-client\26.3\horizon-resources.json'
if (-not (Test-Path -LiteralPath $resources -PathType Leaf)) { Write-Warning "Resources missing: $resources. Re-run with -Prepare and -ServerJar, or use -Resources when launching." }
Write-Output "Horizon: $Horizon"
Write-Output "Run: & `"$project\tools\play-item-state.ps1`" -Inspect"
