# Native Mixins playtest. All generated files and saves live in a separate folder.
$ErrorActionPreference = 'Stop'
$Repo = Split-Path -Parent $PSScriptRoot
$HomeDir = Join-Path $env:LOCALAPPDATA 'TConstruct-Horizon-Playtest'
function Run([string]$Command, [string[]]$Arguments) {
    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Command failed with exit code $LASTEXITCODE" }
}
function Pick-Folder([string]$Title) {
    Add-Type -AssemblyName System.Windows.Forms
    $dialog = New-Object System.Windows.Forms.FolderBrowserDialog
    $dialog.Description = $Title
    if ($dialog.ShowDialog() -ne [System.Windows.Forms.DialogResult]::OK) { throw 'Required folder was not selected.' }
    return $dialog.SelectedPath
}
function Checkout-Pinned([string]$Path, [string]$Url, [string]$Revision) {
    if (-not (Test-Path -LiteralPath $Path)) {
        Run git @('clone', '--no-checkout', $Url, $Path)
        Run git @('-C', $Path, 'checkout', '--detach', $Revision)
    }
    $actual = (& git -C $Path rev-parse HEAD 2>$null)
    if ($LASTEXITCODE -ne 0 -or $actual -ne $Revision) { throw "Pinned source checkout is incomplete or differs: $Path. Preserve any edits and fix this checkout before retrying." }
}
try {
    foreach ($name in @('git', 'cargo', 'py')) {
        if (-not (Get-Command $name -ErrorAction SilentlyContinue)) { throw "$name is missing. Install Git, Python 3.12 and Rust stable MSVC with Visual Studio C++ Build Tools, then try again. Nothing was installed automatically." }
    }
    Run py @('-3', '-c', 'import sys; sys.exit(0 if sys.version_info >= (3, 12) else 1)')
    $compat = Get-Content -LiteralPath (Join-Path $Repo 'compatibility.json') -Raw | ConvertFrom-Json
    $settingsPath = Join-Path $HomeDir 'launcher-settings.json'
    $settings = $null
    if (Test-Path -LiteralPath $settingsPath -PathType Leaf) { $settings = Get-Content -LiteralPath $settingsPath -Raw | ConvertFrom-Json }
    $candidates = @(
        (Join-Path $env:APPDATA 'horizon-launcher\versions\v0.2.66\.local\horizon-client\26.3'),
        (Join-Path $env:LOCALAPPDATA 'horizon-client\26.3'),
        'A:\Project\horizon\.local\horizon-client\26.3',
        (Join-Path (Split-Path -Parent $Repo) 'horizon\.local\horizon-client\26.3')
    )
    if ($settings -and $settings.resources) { $candidates = @($settings.resources) + $candidates }
    $candidates += @(Get-ChildItem -Path (Join-Path $env:APPDATA 'horizon-launcher\versions\*\.local\horizon-client\26.3') -Directory -ErrorAction SilentlyContinue | ForEach-Object FullName)
    $resources = $candidates | Where-Object { Test-Path -LiteralPath (Join-Path $_ 'horizon-resources.json') -PathType Leaf } | Select-Object -First 1
    if (-not $resources) { $resources = Pick-Folder 'Select your prepared Horizon 26.3 resource folder' }
    if (-not (Test-Path -LiteralPath (Join-Path $resources 'horizon-resources.json') -PathType Leaf)) { throw 'The selected folder is not a prepared Horizon 26.3 resource folder. Prepare it with Horizon Launcher first.' }
    # Key the generated build by actual input bytes, so a ZIP download and local
    # modifications work without writing to or resetting the user's checkout.
    $inputs = foreach ($dir in @('mixins', 'content', 'assets', 'tools', 'third_party')) {
        Get-ChildItem -LiteralPath (Join-Path $Repo $dir) -Recurse -File |
            Where-Object { $_.FullName -notmatch '[\\/]__pycache__[\\/]' -and $_.Extension -ne '.pyc' }
    }
    $lines = @($resources) + @($inputs | Sort-Object FullName | ForEach-Object {
        $_.FullName.Substring($Repo.Length) + ':' + (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
    }) + @((Get-FileHash -LiteralPath (Join-Path $Repo 'compatibility.json') -Algorithm SHA256).Hash)
    $hasher = [System.Security.Cryptography.SHA256]::Create()
    try { $head = ([BitConverter]::ToString($hasher.ComputeHash([Text.Encoding]::UTF8.GetBytes(($lines -join "`n"))))).Replace('-', '').ToLowerInvariant() }
    finally { $hasher.Dispose() }
    New-Item -ItemType Directory -Force -Path $HomeDir | Out-Null
    if (-not (Test-Path -LiteralPath $settingsPath)) { @{ resources = $resources } | ConvertTo-Json | Set-Content -LiteralPath $settingsPath -Encoding UTF8 }
    $sources = Join-Path $HomeDir 'sources'
    New-Item -ItemType Directory -Force -Path $sources | Out-Null
    $horizon = Join-Path $sources ('horizon-' + $compat.horizon_revision)
    $mixins = Join-Path $sources ('mixins-' + $compat.mixins_revision)
    Checkout-Pinned $horizon $compat.horizon_repository $compat.horizon_revision
    Checkout-Pinned $mixins $compat.mixins_repository $compat.mixins_revision
    $build = Join-Path $HomeDir ('builds\' + $head)
    $ready = Join-Path $build '.ready'
    $game = Join-Path $build 'horizon-client.exe'
    $gameResources = Join-Path $build 'resources'
    if (-not (Test-Path -LiteralPath $ready)) {
        if (Test-Path -LiteralPath $build) { throw "An incomplete previous build exists at $build. It was left untouched; remove only this generated build folder after checking it, then retry." }
        New-Item -ItemType Directory -Force -Path $build | Out-Null
        Push-Location $Repo
        try {
            Run py @('-3', 'tools/stage.py', '--horizon', $horizon, '--mixins', $mixins, '--output-dir', $build)
            $cargoTarget = Join-Path $HomeDir 'cargo-cache'
            Run cargo @('build', '--locked', '--manifest-path', (Join-Path $build 'modded\Cargo.toml'), '-p', 'horizon-client', '--bin', 'horizon-client', '--no-default-features', '--profile', 'fast', '--target-dir', $cargoTarget)
            Copy-Item -LiteralPath (Join-Path $cargoTarget 'fast\horizon-client.exe') -Destination $game
            $overlay = Join-Path $build 'item-resources'
            Run py @('-3', 'tools/resources.py', $resources, $overlay)
            Run $game @('--tconstruct-prepare', $overlay, (Join-Path $Repo 'content'), $gameResources)
            Run py @('-3', 'tools/prepare_horizon_source_data.py', $resources, (Join-Path $gameResources 'minecraft-original-pack'))
            Run py @('-3', 'tools/prepare_bundled_forge.py', (Join-Path $gameResources 'forge-original-pack'))
            Set-Content -LiteralPath $ready -Value $head
        } finally { Pop-Location }
    }
    if (-not (Test-Path -LiteralPath $game -PathType Leaf) -or -not (Test-Path -LiteralPath (Join-Path $gameResources 'horizon-resources.json') -PathType Leaf)) { throw 'Cached build is incomplete. It was left untouched.' }
    $GameDir = Join-Path $HomeDir 'game'
    New-Item -ItemType Directory -Force -Path $GameDir | Out-Null
    $env:HORIZON_HOME = $GameDir
    Write-Host "Starting isolated world TConstruct-Playtest. Saves: $GameDir\saves"
    Push-Location $GameDir
    try { Run $game @('--resources', $gameResources, '--world', 'TConstruct-Playtest', '--seed', '42', '--creative') } finally { Pop-Location }
} catch {
    Write-Host "`nCould not start the Mixins playtest: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host 'Existing saves and original resources were not changed. See docs/windows-playtest.md for prerequisites and troubleshooting.'
    exit 1
}
