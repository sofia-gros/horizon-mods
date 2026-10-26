param(
    [string]$Client = "$env:APPDATA\horizon-launcher\versions\v0.2.66\horizon-client.exe",
    [string]$Resources = "$env:APPDATA\horizon-launcher\versions\v0.2.66\.local\horizon-client\26.3",
    [switch]$ValidateOnly
)
$ErrorActionPreference = 'Stop'
$project = Split-Path -Parent $PSScriptRoot
$package = Join-Path $project 'target\horizon-mod\tconstruct.hmod'
foreach ($required in @($Client, (Join-Path $Resources 'horizon-resources.json'), $package)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
        throw "Missing file: $required. Build the Mod first; see README.md."
    }
}
$Client = (Resolve-Path -LiteralPath $Client).Path
$Resources = (Resolve-Path -LiteralPath $Resources).Path
$package = (Resolve-Path -LiteralPath $package).Path
Write-Output "Client: $Client"
Write-Output "Package: $package"
Write-Output "Package SHA256: $((Get-FileHash -LiteralPath $package -Algorithm SHA256).Hash)"
if ($ValidateOnly) { return }

$run = Join-Path (Split-Path -Parent $project) ('.local\manual-M1-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $run | Out-Null
$saved = @{}
$settings = @{
    HORIZON_HOME = (Join-Path $run 'game')
    HORIZON_LAUNCHER_HOME = (Join-Path $run 'launcher')
    XDG_DATA_HOME = (Join-Path $run 'xdg-data')
    XDG_CONFIG_HOME = (Join-Path $run 'xdg-config')
    HORIZON_LAUNCHER_NO_SELF_UPDATE = '1'
    HORIZON_NO_AUDIO = '1'
    HORIZON_NO_DISCORD = '1'
    HORIZON_TELEMETRY_URL = 'off'
}
foreach ($key in $settings.Keys) {
    $saved[$key] = [Environment]::GetEnvironmentVariable($key, 'Process')
}
try {
    foreach ($key in $settings.Keys) {
        [Environment]::SetEnvironmentVariable($key, $settings[$key], 'Process')
    }
    foreach ($key in @('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME')) {
        New-Item -ItemType Directory -Path $settings[$key] | Out-Null
    }
    $gameArgs = @('--resources', $Resources, '--dev-mod', $package,
        '--world', 'TConstruct-M1', '--creative', '--seed', '42', '--test-terrain', '--in-frame',
        '--render-distance', '4', '--width', '1280', '--height', '720')
    Write-Output "Isolated test data and game.log: $run"
    Write-Output 'Open chat with T and enter /tconstruct stats wood wood wood'
    Write-Output 'Expected: durability=60, mining=2.000, damage=1.000, speed=1.200'
    # Start-Process joins arguments on Windows; quote each value so the Mod's
    # folder name (with spaces and an apostrophe) stays a single argument.
    $quotedArgs = $gameArgs | ForEach-Object { '"' + $_ + '"' }
    $game = Start-Process -FilePath $Client -ArgumentList $quotedArgs -WorkingDirectory $run `
        -WindowStyle Normal -Wait -PassThru -RedirectStandardOutput (Join-Path $run 'game.log') `
        -RedirectStandardError (Join-Path $run 'game-error.log')
    if ($game.ExitCode -ne 0) { throw "Game exited with code $($game.ExitCode); see $run" }
} finally {
    foreach ($key in $settings.Keys) {
        [Environment]::SetEnvironmentVariable($key, $saved[$key], 'Process')
    }
}
