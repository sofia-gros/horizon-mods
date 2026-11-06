# Explicit latest source playtest. No checkout, install, download or automatic asset search.
param(
    [string]$HorizonSource,
    [string]$MixinsSource,
    [string]$PlutoBundle,
    [string]$Resources,
    [string]$Toolchain,
    [string]$RustfmtArchive,
    [switch]$BuildOnly,
    [string]$PlaytestHome = (Join-Path $env:LOCALAPPDATA 'TConstruct-Horizon-Playtest')
)
$ErrorActionPreference = 'Stop'
function Pick-Folder([string]$Title) {
    Add-Type -AssemblyName System.Windows.Forms
    $dialog = New-Object System.Windows.Forms.FolderBrowserDialog
    $dialog.Description = $Title
    try {
        if ($dialog.ShowDialog() -ne [System.Windows.Forms.DialogResult]::OK) { throw 'Required explicit folder was not selected.' }
        return $dialog.SelectedPath
    } finally { $dialog.Dispose() }
}
function Pick-RustfmtArchive {
    Add-Type -AssemblyName System.Windows.Forms
    $dialog = New-Object System.Windows.Forms.OpenFileDialog
    $dialog.Title = 'Select the existing official Rust 1.99.1 Windows GNU rustfmt tar.gz (no automatic install)'
    $dialog.Filter = 'Rustfmt archive (*.tar.gz)|*.tar.gz'
    $dialog.CheckFileExists = $true
    try {
        if ($dialog.ShowDialog() -ne [System.Windows.Forms.DialogResult]::OK) { throw 'Required explicit rustfmt archive was not selected.' }
        return $dialog.FileName
    } finally { $dialog.Dispose() }
}
try {
    if (-not (Get-Command py -ErrorAction SilentlyContinue)) { throw 'Python 3.12 or newer is required. Nothing was installed automatically.' }
    & py -3 -c 'import sys; sys.exit(0 if sys.version_info >= (3, 12) else 1)'
    if ($LASTEXITCODE -ne 0) { throw 'Python 3.12 or newer is required.' }
    if (-not $HorizonSource) { $HorizonSource = Pick-Folder 'Select clean official Horizon 41f822b source checkout (no fork)' }
    if (-not $MixinsSource) { $MixinsSource = Pick-Folder 'Select clean official Horizon Mixins 5a64af93 source checkout' }
    $BundleArguments = @()
    if ($PlutoBundle) { $BundleArguments = @('--pluto-bundle', $PlutoBundle) }
    $ResourceArguments = @()
    if ($BuildOnly) {
        if ($Resources) { throw 'BuildOnly does not accept Resources. Omit that argument; no game will be started.' }
        $ResourceArguments = @('--build-only')
    } else {
        if (-not $Resources) { $Resources = Pick-Folder 'Select explicitly prepared official Horizon 26.3 resources (not saves)' }
        $ResourceArguments = @('--resources', $Resources)
    }
    if (-not $Toolchain) { $Toolchain = Pick-Folder 'Select official launcher Rust 1.99.1 Windows GNU toolchain root containing bin and mingw64' }
    if (-not $RustfmtArchive) { $RustfmtArchive = Pick-RustfmtArchive }
    & py -3 (Join-Path $PSScriptRoot 'play_windows_latest.py') --horizon $HorizonSource --mixins $MixinsSource --toolchain $Toolchain --rustfmt-archive $RustfmtArchive --home $PlaytestHome @BundleArguments @ResourceArguments
    if ($LASTEXITCODE -ne 0) { throw "Latest guarded playtest stopped with exit code $LASTEXITCODE. No fallback build or guard bypass was used." }
} catch {
    Write-Host ("Could not start latest playtest: " + $_.Exception.Message) -ForegroundColor Red
    Write-Host 'See docs/windows-latest-launch.md. This source route is not yet Windows/GPU verified.'
    exit 1
}
