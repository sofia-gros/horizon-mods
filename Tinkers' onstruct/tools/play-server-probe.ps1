param(
    [string]$Client = 'A:\Project\horizon\target\fast\horizon-client.exe',
    [string]$Resources = "$env:APPDATA\horizon-launcher\versions\v0.2.66\.local\horizon-client\26.3",
    [switch]$InFrame,
    [switch]$ValidateOnly
)
$ErrorActionPreference = 'Stop'
$project = Split-Path -Parent $PSScriptRoot
$clientMod = Join-Path $project 'target\horizon-mod\tconstruct.hmod'
$serverMod = Join-Path $project 'probes\server-lifecycle\target\horizon-mod\tconstruct_probe.hmod'
foreach ($path in @($Client, (Join-Path $Resources 'horizon-resources.json'), $clientMod, $serverMod)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing: $path. See M2 build instructions." }
}
$Client = (Resolve-Path -LiteralPath $Client).Path
$Resources = (Resolve-Path -LiteralPath $Resources).Path
Write-Output "Game: $Client"
Write-Output "Server probe SHA256: $((Get-FileHash -LiteralPath $serverMod -Algorithm SHA256).Hash)"
if ($ValidateOnly) { return }
$run = Join-Path (Split-Path -Parent $project) ('.local\manual-M2-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $run | Out-Null
$probeCopy = Join-Path $run 'tconstruct_probe.hmod'
Copy-Item -LiteralPath $serverMod -Destination $probeCopy
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
$saved = @{}
foreach ($key in $settings.Keys) { $saved[$key] = [Environment]::GetEnvironmentVariable($key, 'Process') }
try {
    foreach ($key in $settings.Keys) { [Environment]::SetEnvironmentVariable($key, $settings[$key], 'Process') }
    foreach ($key in @('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME')) {
        New-Item -ItemType Directory -Path $settings[$key] | Out-Null
    }
    $gameArgs = @('--resources', $Resources, '--dev-mod', $clientMod, '--dev-mod', $probeCopy,
        '--world', 'TConstruct-M2', '--name', 'M2Tester', '--creative', '--seed', '42', '--test-terrain',
        '--render-distance', '4', '--width', '1280', '--height', '720')
    if ($InFrame) { $gameArgs += '--in-frame' }
    Write-Output "Test data and logs: $run"
    Write-Output 'Open chat with T; /tprobe status, /tprobe tell, /tprobe broadcast'
    Write-Output 'Expected: joins=1, online contains your actor, ticks increases between status calls.'
    $quotedArgs = $gameArgs | ForEach-Object { '"' + $_ + '"' }
    $game = Start-Process -FilePath $Client -ArgumentList $quotedArgs -WorkingDirectory $run `
        -WindowStyle Normal -Wait -PassThru -RedirectStandardOutput (Join-Path $run 'game.log') `
        -RedirectStandardError (Join-Path $run 'game-error.log')
    if ($game.ExitCode -ne 0) { throw "Game exited $($game.ExitCode); see $run" }
} finally {
    foreach ($key in $settings.Keys) { [Environment]::SetEnvironmentVariable($key, $saved[$key], 'Process') }
}
