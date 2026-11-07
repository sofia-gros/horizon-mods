#!/usr/bin/env python3
"""Export pinned Horizon sources and apply a resolved Mixins revision."""
import argparse, json, os, re, shutil, subprocess, sys, tempfile, tomllib
from portable_archive import extract_git_tar
import stage_package_contract as package_contract
import stage_float_range_repair
from pathlib import Path

root = Path(__file__).resolve().parents[1]
compat = json.loads((root / 'compatibility.json').read_text())
p = argparse.ArgumentParser()
p.add_argument('--horizon', type=Path, required=True)
p.add_argument('--mixins', type=Path, required=True)
p.add_argument('--mixins-revision', help='full Mixins commit SHA; defaults to the revision in compatibility.json')
p.add_argument('--mods', type=Path, default=root / 'mixins', help='directory of Mixins bundles (defaults to this branch’s TConstruct bundle)')
p.add_argument('--pluto-bundle', type=Path, required=True, help='installed PlutoLib unified Mixins bundle directory')
p.add_argument('--tconstruct-archive', type=Path, required=True, help='absolute same-checkout TConstruct hnmod archive')
p.add_argument('--pluto-archive', type=Path, required=True, help='absolute same-checkout Pluto hnmod archive')
p.add_argument('--tconstruct-archive-sha256', required=True, help='expected SHA256 from the verified package receipt')
p.add_argument('--pluto-archive-sha256', required=True, help='expected SHA256 from the verified package receipt')
p.add_argument('--output-dir', type=Path, default=root / 'target', help='fresh output directory')
p.add_argument('--cli-target-dir', type=Path, help='explicit isolated Mixins compiler output directory')
a = p.parse_args()
head = subprocess.check_output(['git', '-C', str(a.horizon), 'rev-parse', 'HEAD'], text=True).strip()
if head != compat['horizon_revision']:
    p.error(f'{a.horizon}: expected {compat["horizon_revision"]}, found {head}')
requested_revision = a.mixins_revision or compat['mixins_revision']
if not re.fullmatch(r'[0-9a-f]{40}', requested_revision):
    p.error('Mixins revision must be a full lowercase commit SHA')
try:
    revision = subprocess.check_output(['git', '-C', str(a.mixins), 'rev-parse', '--verify', requested_revision + '^{commit}'], text=True, stderr=subprocess.DEVNULL).strip()
except subprocess.CalledProcessError:
    p.error(f'{a.mixins}: required Mixins revision {requested_revision} is not present; fetch it before staging')
print(f'Horizon Mixins revision: {revision}', flush=True)
output = a.output_dir.resolve()
output.mkdir(parents=True, exist_ok=True)
mods = a.mods.resolve(strict=True)
pluto = a.pluto_bundle.resolve()
manifest = pluto / 'horizon-mixin.toml'
try:
    data = tomllib.loads(manifest.read_text())
except (OSError, tomllib.TOMLDecodeError) as error:
    p.error(f'{manifest}: {error}')
if data.get('mod', {}).get('id') != 'pluto_native_creative' or data.get('compatibility', {}).get('horizon_revision') != head:
    p.error(f'{manifest}: PlutoLib bundle id or Horizon revision does not match {head}')
legacy = ('pluto_native_creative', 'pluto_transport_3a', 'pluto_tool_rules', 'pluto_component_display_3a', 'pluto_component_bar', 'pluto_items_b2', 'pluto_creative', 'pluto_items')
for name in legacy:
    if (mods / name).exists():
        p.error(f'{name}: duplicate or legacy PlutoLib bundle; supply only --pluto-bundle')
for candidate in mods.iterdir():
    mixin = candidate / 'horizon-mixin.toml'
    if not mixin.is_file():
        continue
    try:
        existing = tomllib.loads(mixin.read_text())
        for crate in existing.get('crate', []):
            crate_path = (candidate / str(crate.get('path', ''))).resolve()
            if not crate_path.is_relative_to(candidate.resolve()):
                p.error(f'{mixin}: crate path escapes its bundle')
            package = tomllib.loads((crate_path / 'Cargo.toml').read_text()).get('package', {}).get('name')
            if package in ('pluto-api', 'pluto-inventory-api', 'pluto-creative-api', 'pluto-item-api', 'pluto-item-api-b2'):
                p.error(f'{mixin}: duplicate PlutoLib API crate')
    except (OSError, tomllib.TOMLDecodeError) as error:
        p.error(f'{mixin}: {error}')
components = ('pluto_transport_3a-', 'pluto_tool_rules-', 'pluto_component_display_3a-', 'pluto_component_bar-', 'pluto_items_b2-')
entries = data.get('module', []) + data.get('injection', [])
if any(not any(str(entry.get('id', '')).startswith(prefix) for entry in entries) for prefix in components):
    p.error('PlutoLib bundle is incomplete; use the installed unified package')
api_count = 0
inventory_count = 0
for crate in data.get('crate', []):
    crate_path = (pluto / str(crate.get('path', ''))).resolve()
    if not crate_path.is_relative_to(pluto):
        p.error('PlutoLib crate path escapes the bundle')
    try:
        package = tomllib.loads((crate_path / 'Cargo.toml').read_text()).get('package', {}).get('name')
    except (OSError, tomllib.TOMLDecodeError) as error:
        p.error(f'{crate_path}: {error}')
    if package in ('pluto-creative-api', 'pluto-item-api', 'pluto-item-api-b2'):
        p.error('legacy PlutoLib API crate conflicts with the shared facade')
    if package == 'pluto-api':
        api_count += 1
    if package == 'pluto-inventory-api':
        inventory_count += 1
if api_count != 1:
    p.error('PlutoLib unified bundle must provide exactly one pluto-api facade')
if inventory_count != 1 or not any(entry.get('id') == 'pluto_native_creative-inventory-export' for entry in data.get('module', [])):
    p.error('PlutoLib unified bundle must provide exactly one inventory API and its public export')
archives = {'tconstruct_native': a.tconstruct_archive, 'pluto_native_creative': a.pluto_archive}
archive_hashes = {'tconstruct_native': a.tconstruct_archive_sha256, 'pluto_native_creative': a.pluto_archive_sha256}
contract_raw, contract_members = package_contract.prepare(
    root.parent / 'pluto-lib/tools/check_recipe_placement_plan.py', archives, archive_hashes, mods, pluto)
contract_path = output / 'crafting-package-contract.json'
package_contract.save(contract_path, contract_raw)
original = output / 'original'
source = output / 'mixins-source'
if original.exists() or (output / 'modded').exists() or source.exists():
    p.error(f'{output}: original, modded or mixins-source already exists; choose a fresh output directory')
original.mkdir(parents=True)
proc = subprocess.Popen(['git', '-C', str(a.horizon), 'archive', '--format=tar', compat['horizon_revision']], stdout=subprocess.PIPE)
extract_git_tar(proc.stdout, original)
if proc.wait():
    raise SystemExit('git archive failed')
(original / '.horizon-revision').write_text(compat['horizon_revision'] + '\n')
# Build only exported committed source, never a possibly modified working tree.
source.mkdir()
proc = subprocess.Popen(['git', '-C', str(a.mixins), 'archive', '--format=tar', revision], stdout=subprocess.PIPE)
extract_git_tar(proc.stdout, source)
if proc.wait():
    raise SystemExit('Mixins git archive failed')
(output / 'mixins-revision.txt').write_text(revision + '\n')
cli_target = a.cli_target_dir.resolve() if a.cli_target_dir else root.parent / 'target/mixins-cli-build'
subprocess.run(['cargo', 'build', '--release', '--locked', '-p', 'horizon-mixins-cli', '--target-dir', str(cli_target)], cwd=source, check=True)
cli = cli_target / 'release' / ('horizon-mixins.exe' if os.name == 'nt' else 'horizon-mixins')
with tempfile.TemporaryDirectory(prefix='tconstruct-mixins-') as temp:
    combined = Path(temp) / 'bundles'
    shutil.copytree(mods, combined)
    shutil.copytree(pluto, combined / "pluto_native_creative")
    # Validate the complete installed composition, not only Pluto's own hooks.
    # The guard uses this checkout's reviewed source and verifies the exact
    # copied helper bytes embedded in the plan before any source is applied.
    package_contract.verify(contract_path, contract_raw, contract_members, archives, archive_hashes, mods, pluto, combined)
    plan = output / 'complete-plan.json'
    subprocess.run([str(cli), 'plan', '--workspace', str(original), '--mods', str(combined), '--out', str(plan)], check=True)
    package_contract.verify(contract_path, contract_raw, contract_members, archives, archive_hashes, mods, pluto, combined)
    subprocess.run([sys.executable, str(root.parent / 'pluto-lib/tools/check_native_plan.py'), str(plan), str(original), str(contract_path)], check=True)
    package_contract.verify(contract_path, contract_raw, contract_members, archives, archive_hashes, mods, pluto, combined)
    subprocess.run([str(cli), 'apply', '--workspace', str(original), '--mods', str(combined), '--out', str(output / 'modded')], check=True)
    stage_float_range_repair.repair(original, output / 'modded', source, plan, horizon_revision=head, mixins_revision=revision)
