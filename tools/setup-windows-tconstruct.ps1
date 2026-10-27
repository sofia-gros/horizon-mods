[CmdletBinding()]
param(
    [string]$Horizon,
    [string]$Resources,
    [int]$Jobs = 2
)
$ErrorActionPreference = 'Stop'
$mods = Split-Path -Parent $PSScriptRoot
if (-not $Horizon) { $Horizon = Join-Path (Split-Path -Parent $mods) 'horizon' }
$project = Join-Path $mods "Tinkers' onstruct"
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

} finally { Pop-Location }
$builder = Join-Path $Horizon 'target\debug\cargo-horizon-mod.exe'
foreach ($dir in @($project, (Join-Path $project 'server-tools'), (Join-Path $project 'probes\server-lifecycle'))) {
    Push-Location $dir
    try { Invoke-Checked $builder @('horizon-mod', 'build') } finally { Pop-Location }
}
if (-not $Resources) { $Resources = Join-Path $Horizon '.local\horizon-client\26.3' }
if (-not (Test-Path -LiteralPath (Join-Path $Resources 'horizon-resources.json') -PathType Leaf)) {
    Write-Warning "Prepared Horizon resources not found: $Resources. Prepare with Horizon Launcher and pass -Resources to the test launcher."
} else { Write-Output "Prepared resources: $Resources" }
Write-Output "Horizon: $Horizon"
Write-Output "Run: & `"$project\tools\play-item-state.ps1`" -Inspect"
