"""Link the real packaged inventory API into bounded host fixtures."""
import json
from pathlib import Path
import tomllib


def inventory_api_identity(repo: Path):
    package = tomllib.loads((repo / 'pluto-lib/mixins/pluto_native_creative/inventory-api/Cargo.toml').read_text())['package']
    return (package['name'], package['version'], None, None)


def add_inventory_api(manifest: Path, repo: Path):
    """Keep one Cargo crate instance instead of copying its registry modules."""
    api = repo / 'pluto-lib/mixins/pluto_native_creative/inventory-api'
    if inventory_api_identity(repo)[0] != 'pluto-inventory-api':
        raise ValueError('Unexpected inventory API package')
    with manifest.open('a') as output:
        output.write('\n[dependencies.pluto-inventory-api]\npath = ' + json.dumps(str(api.resolve())) + '\n')


def verify_fixture_lock(manifest: Path, host: Path, repo: Path):
    """Accept only the exact host lock and the two known local packages."""
    def identity(package):
        return tuple(package.get(key) for key in ('name', 'version', 'source', 'checksum'))
    fixture = tomllib.loads(manifest.read_text())['package']
    allowed = {identity(package) for package in tomllib.loads((host / 'Cargo.lock').read_text())['package']}
    allowed.add((fixture['name'], fixture['version'], None, None))
    allowed.add(inventory_api_identity(repo))
    unknown = [identity(package) for package in tomllib.loads(manifest.with_name('Cargo.lock').read_text())['package']
               if identity(package) not in allowed]
    if unknown:
        raise ValueError(f'Fixture dependencies differ from official host lock: {unknown}')


def lock_fixture_to_host(manifest: Path, host: Path, repo: Path):
    """Verify the selected revision and resolve without drifting host packages."""
    import subprocess
    expected = json.loads((repo / 'tconstruct/compatibility.json').read_text())['horizon_revision']
    actual = subprocess.check_output(['git', '-C', str(host), 'rev-parse', 'HEAD'], text=True).strip()
    dirty = subprocess.check_output(['git', '-C', str(host), 'status', '--porcelain', '--untracked-files=no'], text=True)
    if actual != expected or dirty:
        raise ValueError(f'HORIZON_HOST must have a clean tracked tree at {expected}')
    manifest.with_name('Cargo.lock').write_text((host / 'Cargo.lock').read_text())
    subprocess.run(['cargo', 'metadata', '--offline', '--manifest-path', str(manifest), '--format-version', '1'],
                   check=True, stdout=subprocess.DEVNULL)
    verify_fixture_lock(manifest, host, repo)


def run_locked_fixture(manifest: Path, required_cases):
    """Run actual fixture tests and require each named boundary test to pass."""
    import subprocess
    result = subprocess.run(['cargo', 'test', '--locked', '--offline', '--manifest-path', str(manifest)],
                            text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    print(result.stdout)
    result.check_returncode()
    lines = result.stdout.splitlines()
    for case in required_cases:
        if f'test {case} ... ok' not in lines:
            raise ValueError(f'Required native fixture case did not pass: {case}')
