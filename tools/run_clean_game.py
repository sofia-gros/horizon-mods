"""Run an explicitly selected, source-bound clean game in a fresh isolated home.

This is the repository development entry, not official-launcher execution or
Tinkers gameplay certification. Never discovers or falls back to old binaries.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import tomllib

from check_game_entry import PIN, source_digest
from port import checked_files

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def checked_executable(prepared, cache_root):
    """Bind successful prerequisite receipts, full staged source and executable."""
    for name in ('contract-check', 'native-check', 'api-check', 'consumer-check', 'game-entry-check'):
        if json.loads((prepared / (name + '.json')).read_text(encoding='utf-8'))['exit'] != 0:
            raise ValueError('failed prerequisite: ' + name)
    proof = json.loads((prepared / 'game-entry-link.json').read_text(encoding='utf-8'))
    if proof.get('exit') != 0 or proof.get('binary_linked') is not True or proof.get('horizon_revision') != PIN:
        raise ValueError('successful pinned executable-link receipt required')
    source = source_digest(prepared / 'applied')
    entry = json.loads((prepared / 'game-entry-check.json').read_text(encoding='utf-8'))
    if source != proof.get('source_sha256') or source != entry.get('source_sha256'):
        raise ValueError('staged source changed since verification')
    executable = Path(proof['executable']).resolve()
    if not executable.is_relative_to(cache_root.resolve()) or executable.is_relative_to(prepared):
        raise ValueError('executable must be in the separate repository-local build cache')
    if digest(executable) != proof.get('binary_sha256'):
        raise ValueError('executable changed since linking')
    return executable, proof


def validate(prepared, resources):
    prepared = prepared.resolve()
    if not prepared.is_relative_to((ROOT / '.local/clean-port').resolve()):
        raise ValueError('prepared input must be an isolated clean-port directory')
    resources = resources.resolve()
    if not (resources / 'horizon-resources.json').is_file():
        raise ValueError('explicit Horizon resource directory required')
    executable, proof = checked_executable(prepared, ROOT / '.local')
    consumer = json.loads((prepared / 'consumer-check.json').read_text(encoding='utf-8'))
    if consumer['consumer_inputs'] != {name: hashlib.sha256(data).hexdigest()
                                      for name, data in checked_files(ROOT)[0].items()}:
        raise ValueError('clean product changed since verification')
    tools = ROOT / 'pluto-lib/tools'
    sys.path.insert(0, str(tools))
    from prepare_direct_host import validate_transferred_declarations, DECLARATION_HASHES
    # Preparation transfers direct declarations out of the companion manifest.
    # Compare that exact projection, validating every removed declaration first.
    current = tomllib.loads((ROOT / 'pluto-lib/mod/horizon-mixin.toml').read_text(encoding='utf-8'))
    validate_transferred_declarations(current)
    for section in ('injection', 'state'):
        current[section] = [entry for entry in current.get(section, [])
                            if entry['id'] not in DECLARATION_HASHES]
    snapshot = tomllib.loads((prepared / 'bundle/horizon-mixin.toml').read_text(encoding='utf-8'))
    if current != snapshot:
        raise ValueError('PlutoLib manifest changed since verification; rebuild first')
    def files(folder):
        return {p.relative_to(folder).as_posix(): digest(p)
                for p in folder.rglob('*') if p.is_file() and p.relative_to(folder).as_posix() != 'horizon-mixin.toml'}
    if files(prepared / 'bundle') != files(ROOT / 'pluto-lib/mod'):
        raise ValueError('PlutoLib bundle changed since verification; rebuild first')
    # Run every current guard again; a stale success receipt is insufficient.
    from check_sliced_plan import GUARDS
    subprocess.run([sys.executable, str(tools / 'check_sliced_plan.py'),
                    str(prepared / 'plan.json'), str(prepared / 'horizon'),
                    ','.join(GUARDS), str(prepared / 'direct-host.json')], check=True)
    return executable, proof, resources


def launch(executable, proof, resources, automation=False, content=False):
    # Never accept an existing home/world path or borrow the user's launcher home.
    run = Path(tempfile.mkdtemp(prefix='clean-game-', dir=ROOT / '.local/clean-port'))
    env = dict(os.environ, HORIZON_NO_DISCORD='1', HORIZON_TELEMETRY_URL='off',
               HORIZON_LAUNCHER_NO_SELF_UPDATE='1', RUST_MIN_STACK='16777216')
    for key in ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'TMP', 'TEMP'):
        folder = run / key.lower()
        folder.mkdir()
        env[key] = str(folder)
    game_dir = run / 'game'
    game_dir.mkdir()
    if content:
        # The clean Tinkers content mod, as a folder with horizon-mod.toml, switched on for this fresh game dir only.
        import shutil
        shutil.copytree(ROOT / 'tconstruct/content', game_dir / 'horizon-mods/tconstruct')
        (game_dir / 'horizon-mods.txt').write_text('mod.tconstruct:true' + chr(10), encoding='utf-8', newline=chr(10))
    command = [str(executable), '--home', env['HORIZON_HOME'], '--game-dir', str(game_dir),
               '--resources', str(resources), '--world', 'CleanPortDevelopment', '--seed', '42', '--creative']
    if automation:
        command += ['--control-socket', str(run / 'control.json'), '--offscreen',
                    '--render-distance', '2', '--width', '1280', '--height', '720']
    print('Development entry only; Tinkers content is not yet playable.' + (' (clean content mod enabled)' if content else ''), flush=True)
    print('Isolated run: ' + str(run), flush=True)
    result = subprocess.run(command, cwd=run, env=env)
    (run / 'launch.json').write_text(newline='\n', data=json.dumps({
        'exit': result.returncode, 'source_sha256': proof['source_sha256'],
        'binary_sha256': proof['binary_sha256'], 'command': command,
        'official_launcher_executed': False, 'gameplay_parity': False,
    }, indent=2) + '\n', encoding='utf-8')
    return result.returncode


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prepared', required=True, type=Path)
    parser.add_argument('--resources', required=True, type=Path)
    parser.add_argument('--check-only', action='store_true', help='Verify inputs without creating a home or launching')
    parser.add_argument('--automation', action='store_true',
                        help='Offscreen development run with a control endpoint inside the fresh isolated home')
    parser.add_argument('--content', action='store_true',
                        help='Enable tconstruct/content as a content mod in the fresh isolated game directory')
    args = parser.parse_args()
    try:
        executable, proof, resources = validate(args.prepared, args.resources)
        if args.check_only:
            print('Verified development executable: ' + str(executable))
            return 0
        return launch(executable, proof, resources, automation=args.automation, content=args.content)
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as error:
        print('Clean game launch refused: ' + str(error), file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
