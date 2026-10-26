param(
    [string]$Client = 'A:\Project\horizon\target\fast\horizon-client.exe',
    [string]$Resources = "$env:APPDATA\horizon-launcher\versions\v0.2.66\.local\horizon-client\26.3",
    [switch]$InFrame,
    [switch]$Inspect,
    [switch]$ResumeLatest,
    [switch]$ValidateOnly
)
$ErrorActionPreference = 'Stop'
$project = Split-Path -Parent $PSScriptRoot
$clientMod = Join-Path $project 'target\horizon-mod\tconstruct.hmod'
$serverMod = Join-Path $project 'server-tools\target\horizon-mod\tconstruct_tools.hmod'
$required = @($Client, (Join-Path $Resources 'horizon-resources.json'), $clientMod)
if ($Inspect) { $required += $serverMod }
foreach ($path in $required) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing: $path. See M3a instructions." }
}
$Client = (Resolve-Path -LiteralPath $Client).Path
$Resources = (Resolve-Path -LiteralPath $Resources).Path
Write-Output "Game: $Client"
Write-Output "Mod SHA256: $((Get-FileHash -LiteralPath $clientMod -Algorithm SHA256).Hash)"
if ($ValidateOnly) { return }
$parent = Join-Path (Split-Path -Parent $project) '.local'
$stage = if ($Inspect) { 'M3b1' } else { 'M3a' }
if ($ResumeLatest) {
    $latest = Get-ChildItem -LiteralPath $parent -Directory -Filter ("manual-$stage-*") |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $latest) { throw "No $stage manual test exists. Run without -ResumeLatest first." }
    $run = $latest.FullName
} else {
    $run = Join-Path $parent ("manual-$stage-" + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $run | Out-Null
}
$settings = @{
    HORIZON_HOME = (Join-Path $run 'game')
    HORIZON_LAUNCHER_HOME = (Join-Path $run 'launcher')
    XDG_DATA_HOME = (Join-Path $run 'xdg-data')
    XDG_CONFIG_HOME = (Join-Path $run 'xdg-config')
    HORIZON_LAUNCHER_NO_SELF_UPDATE = '1'
    HORIZON_NO_AUDIO = '1'
    HORIZON_NO_DISCORD = '1'
    HORIZON_TELEMETRY_URL = 'off'
    RUST_MIN_STACK = '16777216'
    TEMP = (Join-Path $run 'tmp')
    TMP = (Join-Path $run 'tmp')
}
$saved = @{}
foreach ($key in $settings.Keys) { $saved[$key] = [Environment]::GetEnvironmentVariable($key, 'Process') }
try {
    foreach ($key in $settings.Keys) { [Environment]::SetEnvironmentVariable($key, $settings[$key], 'Process') }
    foreach ($key in @('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'TEMP')) {
        New-Item -ItemType Directory -Path $settings[$key] -Force | Out-Null
    }
    $gameArgs = @('--resources', $Resources, '--dev-mod', $clientMod, '--control-socket', (Join-Path $run 'control.json'),
        '--world', "TConstruct-$stage", '--name', 'M3Tester', '--creative', '--seed', '42', '--test-terrain',
        '--render-distance', '4', '--width', '1280', '--height', '720')
    if ($InFrame) { $gameArgs += '--in-frame' }
    if ($Inspect) { $gameArgs += @('--dev-mod', $serverMod) }
    Write-Output "Test data and logs: $run"
    Write-Output '/tconstruct specimen wood wood wood'
    Write-Output '/tconstruct specimen flint wood wood 17'
    Write-Output '/tconstruct specimen flint wood wood 85'
    Write-Output 'Expected: three vanilla pickaxes named TConstruct state specimen. Gameplay remains vanilla.'
    if ($Inspect) {
        Write-Output 'Select each specimen, then /tconstruct_tool inspect. Server report: damage=0 / 17 / 85, broken=false / false / true, hooks=false.'
        Write-Output 'Creative core-state tests: /tconstruct_tool test-damage 100; /tconstruct_tool test-repair 20. These do not enable gameplay durability hooks.'
    }
    if ($Inspect) { Write-Output 'Quit, then run this script with -Inspect -ResumeLatest to reopen this test world.' }
    else { Write-Output 'Quit, then run this script with -ResumeLatest to reopen this test world.' }
    $quotedArgs = $gameArgs | ForEach-Object { '"' + $_ + '"' }
    $game = Start-Process -FilePath $Client -ArgumentList $quotedArgs -WorkingDirectory $run `
        -WindowStyle Normal -Wait -PassThru -RedirectStandardOutput (Join-Path $run 'game.log') `
        -RedirectStandardError (Join-Path $run 'game-error.log')
    if ($game.ExitCode -ne 0) { throw "Game exited $($game.ExitCode); see $run" }
} finally {
    foreach ($key in $settings.Keys) { [Environment]::SetEnvironmentVariable($key, $saved[$key], 'Process') }
}
