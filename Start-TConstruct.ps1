# Launch an already built TConstruct-enabled Horizon client on Windows.
# Saves and settings are isolated from the normal Horizon Launcher installation.
param(
  [switch]$CheckOnly
)

$ErrorActionPreference = 'Stop'

try {
  $repo = $PSScriptRoot
  $target = Join-Path $repo 'target'
  if (-not (Test-Path -LiteralPath $target -PathType Container)) {
    throw 'The prebuilt Horizon game is missing. Build TConstruct first.'
  }

  $builds = @(Get-ChildItem -LiteralPath $target -Directory -Filter 'win-main-*' |
    Sort-Object LastWriteTime -Descending)
  $selected = $null
  foreach ($build in $builds) {
    $game = Join-Path $build.FullName 'fast\horizon-client.exe'
    $resources = Join-Path $build.FullName 'game-resources'
    $manifest = Join-Path $resources 'horizon-resources.json'
    if ((Test-Path -LiteralPath $game -PathType Leaf) -and
        (Test-Path -LiteralPath $manifest -PathType Leaf)) {
      $selected = $build
      break
    }
  }
  if (-not $selected) {
    throw 'No complete Windows game build was found under target\win-main-*. Build TConstruct first.'
  }

  $game = Join-Path $selected.FullName 'fast\horizon-client.exe'
  $resources = Join-Path $selected.FullName 'game-resources'
  $builtRevision = $selected.Name -replace '^win-main-', ''
  Write-Host "Horizon + TConstruct build: $builtRevision" -ForegroundColor Cyan
  Write-Host "Game: $game"
  Write-Host "Resources: $resources"

  if (Get-Command git -ErrorAction SilentlyContinue) {
    $currentRevision = & git -C $repo rev-parse --short=8 HEAD 2>$null
    if ($LASTEXITCODE -eq 0 -and $currentRevision -and
        $currentRevision.Trim() -ne $builtRevision) {
      Write-Warning "Source is at $($currentRevision.Trim()), but this executable was built from $builtRevision. Rebuild to play merged changes."
    }
  }

  if ($CheckOnly) {
    Write-Host 'Launch files verified. Ready to play.' -ForegroundColor Green
    exit 0
  }

  $playRoot = Join-Path $env:LOCALAPPDATA 'TConstruct-Horizon-Playtest\one-click'
  New-Item -ItemType Directory -Force -Path $playRoot | Out-Null
  foreach ($name in @('horizon-home', 'launcher-home', 'data-home', 'config-home')) {
    New-Item -ItemType Directory -Force -Path (Join-Path $playRoot $name) | Out-Null
  }

  $env:HORIZON_HOME = Join-Path $playRoot 'horizon-home'
  $env:HORIZON_LAUNCHER_HOME = Join-Path $playRoot 'launcher-home'
  $env:XDG_DATA_HOME = Join-Path $playRoot 'data-home'
  $env:XDG_CONFIG_HOME = Join-Path $playRoot 'config-home'
  $env:HORIZON_NO_AUDIO = '1'
  $env:HORIZON_NO_DISCORD = '1'
  $env:HORIZON_TELEMETRY_URL = 'off'
  $env:RUST_MIN_STACK = '16777216'

  Write-Host "Saves: $($env:HORIZON_HOME)\saves"
  Write-Host 'Starting TConstruct-Playtest (Creative mode)...' -ForegroundColor Green
  Push-Location -LiteralPath $playRoot
  try {
    & $game '--resources' $resources '--world' 'TConstruct-Playtest' '--seed' '42' '--creative'
    if ($LASTEXITCODE -ne 0) {
      throw "Horizon exited with code $LASTEXITCODE"
    }
  } finally {
    Pop-Location
  }
} catch {
  Write-Host "Launch failed: $($_.Exception.Message)" -ForegroundColor Red
  exit 1
}
