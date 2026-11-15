# Repository development entry, not a playable Tinkers release.
# Explicit verified inputs only; each launch creates a fresh isolated home/world.
param(
  [Parameter(Mandatory = $true)][string]$Prepared,
  [Parameter(Mandatory = $true)][string]$Resources,
  [switch]$CheckOnly,
  [switch]$Automation
)

$ErrorActionPreference = 'Stop'
try {
  $launcher = Join-Path $PSScriptRoot 'tools/run_clean_game.py'
  $launchArgs = @($launcher, '--prepared', $Prepared, '--resources', $Resources)
  if ($CheckOnly) { $launchArgs += '--check-only' }
  if ($Automation) { $launchArgs += '--automation' }
  & python @launchArgs
  exit $LASTEXITCODE
} catch {
  Write-Error "Clean development launch failed: $($_.Exception.Message)"
  exit 1
}
