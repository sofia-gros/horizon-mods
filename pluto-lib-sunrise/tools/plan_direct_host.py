"""Plan all PlutoLib attachments on a reproducible, directly patched Horizon copy."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tomllib


def stage_api_check(stage, bundle):
    """Diagnostic public-API check, not the launcher's executable integration.

    Mirror the pinned launcher native_mods/api.rs path rewrite in an isolated
    stage. Keep its package/features and the workspace's dependency patches.
    """
    source = bundle / 'api'
    manifest = (source / 'Cargo.toml').read_text(encoding='utf-8')
    parsed = tomllib.loads(manifest)
    dependency = parsed['dependencies']['horizon-client']
    if dependency != {'git': 'https://github.com/sunrise-org/sunrise',
                      'rev': '441f49d57e344bd56b90fd4af5c4e443ff3937c9',
                      'default-features': False}:
        raise ValueError('API dependency changed; review the staged host rewrite')
    lines = manifest.splitlines(keepends=True)
    matching = [i for i, line in enumerate(lines) if line.startswith('horizon-client = ')]
    if len(matching) != 1:
        raise ValueError('expected exactly one API host dependency')
    lines[matching[0]] = 'horizon-client = { path = "../../crates/horizon-client", default-features = false }\n'
    workspace = stage / 'Cargo.toml'
    root = workspace.read_text(encoding='utf-8')
    if root.count('members = [\n') != 1:
        raise ValueError('workspace member declaration changed')
    dest = stage / 'native-api/pluto_lib'
    if dest.exists():
        raise ValueError('API check input already exists')
    shutil.copytree(source, dest)
    # The API package re-exports the movement registry crate as
    # `pluto_lib::movement` through a path dependency
    # (../../mixins/pluto_movement/api from the package). Stage that crate at
    # the same relative place from the staged package so the path resolves
    # exactly as it does in the repository tree.
    movement_dest = stage / 'mixins/pluto_movement/api'
    if movement_dest.exists():
        raise ValueError('movement API staging already exists')
    shutil.copytree(Path(__file__).resolve().parents[1] / 'mixins/pluto_movement/api',
                    movement_dest)
    (dest / 'Cargo.toml').write_text(''.join(lines), encoding='utf-8', newline='\n')
    workspace.write_text(newline='\n', data=root.replace('members = [\n', 'members = [\n    "native-api/pluto_lib",\n', 1),
                         encoding='utf-8')
    return parsed['package']['name']

from prepare_direct_host import prepare
from check_sliced_plan import GUARDS


def checkout_source(source, workspace, revision):
    """Materialize pinned Git blobs as LF regardless of the user's Git config."""
    config = ['-c', 'core.autocrlf=false', '-c', 'core.eol=lf', '-c', 'core.symlinks=false']
    subprocess.run(['git', *config, 'clone', '--shared', '--no-checkout',
                    '--config', 'core.autocrlf=false', '--config', 'core.eol=lf',
                    '--config', 'core.symlinks=false', str(source), str(workspace)], check=True)
    subprocess.run(['git', '-C', str(workspace), *config, 'checkout', '--detach', revision], check=True)


def check_contracts(output):
    """Run every retained guard, including mixed direct-source ownership checks.

    Do not select a smaller set to make an unmigrated production contract pass.
    A failing guard blocks apply/compile but leaves the complete plan available.
    """
    command = [sys.executable, str(Path(__file__).with_name('check_sliced_plan.py')),
               str(output / 'plan.json'), str(output / 'horizon'), ','.join(GUARDS),
               str(output / 'direct-host.json')]
    result = subprocess.run(command, capture_output=True, text=True)
    (output / 'contract-check.json').write_text(newline='\n', data=json.dumps({
        'command': command, 'exit': result.returncode,
        'stdout': result.stdout, 'stderr': result.stderr,
    }, indent=2) + '\n', encoding='utf-8')
    print(result.stdout, end='')
    print(result.stderr, end='', file=sys.stderr)
    return result.returncode


def plan(source, output, mixins, check_native=False, check_api=False, require_contracts=False):
    source, output, mixins = source.resolve(), output.resolve(), mixins.resolve()
    revision = subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip()
    if revision != '441f49d57e344bd56b90fd4af5c4e443ff3937c9':
        raise ValueError('reference checkout is not the pinned Horizon revision')
    receipt = prepare(source, output)
    workspace = output / 'horizon'
    # A real checkout is required: the CLI resolves the workspace's Git HEAD.
    # Materialize tracked documentation links as ordinary files on Windows.
    checkout_source(source, workspace, revision)
    shutil.copytree(output / 'horizon-overlay', workspace, dirs_exist_ok=True)
    cli_bundle = output / 'mods/pluto_lib'
    shutil.copytree(output / 'bundle', cli_bundle)
    manifest = cli_bundle / 'sunrise-mod.toml'
    lines, skip = [], False
    for line in manifest.read_text(encoding='utf-8').splitlines(keepends=True):
        if line.strip().startswith('['):
            skip = line.strip() in ('[api]', '[dependencies]')
        if not skip:
            lines.append(line)
    # The CLI understands attachments only. Keep launcher metadata in bundle/;
    # this projection removes no injection, module, bridge or remaining state.
    manifest.write_text(''.join(lines), encoding='utf-8', newline='\n')
    command = [str(mixins), 'plan', '--workspace', str(workspace),
               '--mods', str(output / 'mods'), '--out', str(output / 'plan.json')]
    result = subprocess.run(command, capture_output=True, text=True)
    (output / 'plan-result.json').write_text(newline='\n', data=json.dumps({
        'command': command, 'revision': receipt['horizon_revision'],
        'exit': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr,
        'native_compiled': False,
    }, indent=2) + '\n', encoding='utf-8')
    print(result.stdout, end='')
    print(result.stderr, end='')
    print(f'Artifacts: {output}', flush=True)
    if result.returncode:
        return result.returncode
    if require_contracts:
        contract_result = check_contracts(output)
        if contract_result:
            return contract_result
    if not check_native:
        return 0
    stage = output / 'applied'
    apply_command = [str(mixins), 'apply', '--workspace', str(workspace),
                     '--mods', str(output / 'mods'), '--out', str(stage)]
    applied = subprocess.run(apply_command)
    if applied.returncode:
        return applied.returncode
    # This compiles host attachments, not the launcher's trait-crate wiring.
    # --keep-going (here and in the sibling check scripts): one run surfaces
    # every independent crate's errors instead of stopping at the first
    # failing crate; the exit code is unchanged.
    checked = subprocess.run(['cargo', '+1.95.0', 'check', '--offline', '--locked',
                              '--manifest-path', str(stage / 'Cargo.toml'), '-p', 'horizon-client',
                              '--keep-going'])
    (output / 'native-check.json').write_text(newline='\n', data=json.dumps({
        'exit': checked.returncode, 'trait_linkage_checked': False,
        'gameplay_checked': False,
    }, indent=2) + '\n', encoding='utf-8')
    if checked.returncode or not check_api:
        return checked.returncode
    package = stage_api_check(stage, output / 'bundle')
    # The shipped composition includes the bundle's native inventory gates
    # (the pluto-inventory module), so the API check must also compile the
    # forwarders that install into them: enable the API package's
    # composition feature here, mirroring the game entry's mod edge. A
    # bundle without the gates keeps the standalone (feature-off) check.
    bundle_manifest = tomllib.loads(
        (output / 'bundle/sunrise-mod.toml').read_text(encoding='utf-8'))
    glue = any(entry.get('id') == 'pluto-inventory'
               for entry in bundle_manifest.get('module', []))
    # Adding the local package requires a lockfile entry. Resolve offline from
    # the copied upstream lock, then check under --locked; never touch the pin.
    api_command = ['cargo', '+1.95.0', 'check', '--offline', '--manifest-path',
                   str(stage / 'Cargo.toml'), '-p', package, '--keep-going']
    if glue:
        api_command += ['--features', 'native-inventory-glue']
    resolved = subprocess.run(api_command)
    api_result = resolved.returncode
    if not api_result:
        api_result = subprocess.run(api_command[:4] + ['--locked'] + api_command[4:]).returncode
    (output / 'api-check.json').write_text(newline='\n', data=json.dumps({
        'exit': api_result, 'package': package,
        'launcher_integration_checked': False, 'gameplay_checked': False,
    }, indent=2) + '\n', encoding='utf-8')
    return api_result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--mixins', required=True, type=Path)
    parser.add_argument('--check-native', action='store_true', help='Apply all attachments and typecheck the native client (not trait linkage or gameplay)')
    parser.add_argument('--check-api', action='store_true', help='Also typecheck the snapshot PlutoLib API against the adapted host (not launcher integration)')
    parser.add_argument('--check-contracts', action='store_true', help='Require every retained production contract before applying or compiling (mandatory in CI)')
    args = parser.parse_args()
    raise SystemExit(plan(args.source, args.output, args.mixins, args.check_native or args.check_api, args.check_api, args.check_contracts))
