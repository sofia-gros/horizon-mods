param(
    [string]$HorizonPath = 'A:\Project\horizon',
    [string]$Base = 'HEAD'
)
$ErrorActionPreference = 'Stop'

# Fetch and inspect only. Integration and validation are separate steps.
& git -C $HorizonPath fetch upstream main
if ($LASTEXITCODE -ne 0) { throw 'upstream fetch failed' }
& git -C $HorizonPath rev-parse --verify "$Base^{commit}"
if ($LASTEXITCODE -ne 0) { throw 'Invalid comparison base' }
& git -C $HorizonPath log --oneline "$Base..upstream/main"
if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect upstream commits' }
$changed = @(& git -C $HorizonPath diff --name-only "$Base...upstream/main")
if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect upstream paths' }
$relevant = @($changed | Where-Object {
    $_ -match '^(Cargo\.(toml|lock)$|crates/(horizon-mods|horizon-mod-sdk|cargo-horizon-mod|horizon-client|horizon-server)/|launcher/|mods/|docs/(api/|mods))'
})
if ($relevant.Count -eq 0) {
    Write-Output 'No Mod-related paths changed since the comparison base.'
} else {
    Write-Output 'Review these paths before integration (client/server/launcher matches are intentionally broad):'
    $relevant | Write-Output
}
& git -C $HorizonPath status --short
if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect working tree' }
