[CmdletBinding()]
param(
    [string]$Horizon,
    [ValidateSet('integrated', 'in-frame', 'players', 'dedicated', 'all')]
    [string]$Mode = 'all'
)
$ErrorActionPreference = 'Stop'
$mods = Split-Path -Parent $PSScriptRoot
if (-not $Horizon) { $Horizon = Join-Path (Split-Path -Parent $mods) 'horizon' }
$python = Get-Command py -ErrorAction SilentlyContinue
if (-not $python) { throw 'Python launcher py not found. Install Python 3 for Windows.' }
$game = Join-Path $Horizon 'target\fast\horizon-client.exe'
$ctl = Join-Path $Horizon 'target\fast\horizon-ctl.exe'
$resources = Join-Path $Horizon '.local\horizon-client\26.3'
$common = @('--game', $game, '--ctl', $ctl, '--resources', $resources)
$checks = if ($Mode -eq 'all') { @('integrated', 'in-frame', 'players', 'dedicated') } else { @($Mode) }
foreach ($check in $checks) {
    $script = if ($check -in @('players', 'dedicated')) { 'probe_horizon_mining_players.py' } else { 'probe_horizon_mining_wear.py' }
    $extra = if ($check -eq 'in-frame') { @('--in-frame') } elseif ($check -eq 'dedicated') { @('--dedicated') } else { @() }
    Write-Output "Running $check"
    & $python.Source -3 (Join-Path $PSScriptRoot $script) @common @extra
    if ($LASTEXITCODE -ne 0) { throw "$check failed with exit code $LASTEXITCODE. See the printed report and game.log." }
}
