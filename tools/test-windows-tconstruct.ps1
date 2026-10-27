[CmdletBinding()]
param(
    [string]$Horizon,
    [string]$Resources,
    [ValidateSet('integrated', 'in-frame', 'players', 'dedicated', 'coexist', 'coexist-in-frame', 'melee', 'melee-in-frame', 'all')]
    [string]$Mode = 'all'
)
$ErrorActionPreference = 'Stop'
$mods = Split-Path -Parent $PSScriptRoot
if (-not $Horizon) { $Horizon = Join-Path (Split-Path -Parent $mods) 'horizon' }
$python = Get-Command py -ErrorAction SilentlyContinue
if (-not $python) { throw 'Python launcher py not found. Install Python 3 for Windows.' }
$game = Join-Path $Horizon 'target\fast\horizon-client.exe'
$ctl = Join-Path $Horizon 'target\fast\horizon-ctl.exe'
if (-not $Resources) { $Resources = Join-Path $Horizon '.local\horizon-client\26.3' }
$common = @('--game', $game, '--ctl', $ctl, '--resources', $resources)
$checks = if ($Mode -eq 'all') { @('integrated', 'in-frame', 'players', 'dedicated', 'coexist', 'coexist-in-frame', 'melee', 'melee-in-frame') } else { @($Mode) }
foreach ($check in $checks) {
    $script = if ($check -in @('players', 'dedicated')) { 'probe_horizon_mining_players.py' } elseif ($check -in @('melee', 'melee-in-frame')) { 'probe_horizon_melee_wear.py' } else { 'probe_horizon_mining_wear.py' }
    $extra = if ($check -in @('in-frame', 'melee-in-frame')) { @('--in-frame') } elseif ($check -eq 'dedicated') { @('--dedicated') } elseif ($check -in @('coexist', 'coexist-in-frame')) {
        $probe = Join-Path $mods "Tinkers' onstruct\probes\server-lifecycle\target\horizon-mod\tconstruct_probe.hmod"
        $arguments = @('--lifecycle-probe', $probe)
        if ($check -eq 'coexist-in-frame') { $arguments += '--in-frame' }
        $arguments
    } else { @() }
    Write-Output "Running $check"
    & $python.Source -3 (Join-Path $PSScriptRoot $script) @common @extra
    if ($LASTEXITCODE -ne 0) { throw "$check failed with exit code $LASTEXITCODE. See the printed report and game.log." }
}
