"""Stage/typecheck the pinned launcher's real game entry in an isolated workspace.

Default mode typechecks. --link links a previously checked unchanged snapshot.
Neither mode launches, publishes, or approves pending attachment contracts.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tomllib

from port import checked_files

ROOT = Path(__file__).resolve().parents[1]
PIN = '441f49d57e344bd56b90fd4af5c4e443ff3937c9'
PACKAGE = 'horizon-native-game'

# launcher/src/native_mods/api.rs at PIN: main_rs and stage's native-game manifest.
# One deliberate composition extension: the pluto_lib edge enables the API
# package's `native-inventory-glue` feature, because the shipped bundle now
# carries the native inventory gates those forwarders install into.
MANIFEST = '''[package]
name = "horizon-native-game"
version = "0.1.0"
edition = "2024"
publish = false
build = "../horizon-client/build.rs"
[[bin]]
name = "horizon-client"
path = "src/main.rs"
[dependencies]
horizon-client = { path = "../horizon-client", default-features = false }
horizon-threads = { path = "../horizon-threads" }
horizon-mods = { path = "../horizon-mods" }
horizon-official = { path = "../horizon-official", optional = true }
mod_0 = { package = "mod-pluto_lib", path = "../../native-api/pluto_lib", features = ["native-inventory-glue"] }
mod_1 = { package = "mod-tconstruct", path = "../../native-api/tconstruct" }
mod_2 = { package = "mod-pipes", path = "../../native-api/pipes" }
[features]
default = ["horizon-client/default", "official"]
official = ["dep:horizon-official"]
[build-dependencies]
horizon-mod-api-gen = { path = "../horizon-mod-api-gen" }
winresource = "0.1"
[target.'cfg(windows)'.build-dependencies]
winresource = "0.1"
'''
MAIN = '''//! Generated for isolated game-entry verification; not a certified game build.
#![forbid(unsafe_code)]
#[global_allocator]
static ALLOCATOR: horizon_threads::GameAllocator = horizon_threads::GAME_ALLOCATOR;
static MOD_API_LEVEL: [u8; horizon_mods::api_level::MARK.len()] = horizon_mods::api_level::MARK;
fn main() {
    std::hint::black_box(&MOD_API_LEVEL);
    let mods: horizon_client::mod_api::Mods = vec![mod_0::hooks(), mod_1::hooks(), mod_2::hooks()].into();
    horizon_client::mod_api::name_mods(&["pluto_lib", "tconstruct", "pipes"]);
    #[cfg(feature = "official")]
    horizon_official::register();
    horizon_client::entry::main_with(mods);
}
'''


def prepare(prepared):
    prepared = prepared.resolve()
    if not prepared.is_relative_to((ROOT / '.local/clean-port').resolve()):
        raise ValueError('game entry requires an isolated clean-port workspace')
    receipt = json.loads((prepared / 'direct-host.json').read_text(encoding='utf-8'))
    if receipt['horizon_revision'] != PIN:
        raise ValueError('game entry requires the inspected Horizon pin')
    for name in ('native-check', 'api-check', 'consumer-check'):
        if json.loads((prepared / (name + '.json')).read_text(encoding='utf-8'))['exit'] != 0:
            raise ValueError('successful host/API/consumer checks required')
    files = checked_files(ROOT)[0]
    consumer = json.loads((prepared / 'consumer-check.json').read_text(encoding='utf-8'))
    if consumer['consumer_inputs'] != {name: hashlib.sha256(data).hexdigest() for name, data in files.items()}:
        raise ValueError('consumer inputs changed since typecheck')
    stage = prepared / 'applied'
    game = stage / 'crates/native-game'
    workspace = stage / 'Cargo.toml'
    original = workspace.read_text(encoding='utf-8')
    if game.exists() or game.is_symlink():
        raise ValueError('game entry already exists; preserve its evidence')
    if original.count('members = [\n') != 1:
        raise ValueError('workspace member declaration changed')
    if not (stage / 'crates/horizon-official/Cargo.toml').is_file():
        raise ValueError('pinned official game package is missing')
    client = tomllib.loads((stage / 'crates/horizon-client/Cargo.toml').read_text(encoding='utf-8'))
    if client.get('build-dependencies') != tomllib.loads(MANIFEST)['build-dependencies']:
        raise ValueError('client build dependencies changed; review launcher projection')
    (game / 'src').mkdir(parents=True)
    (game / 'Cargo.toml').write_text(MANIFEST, encoding='utf-8', newline='\n')
    (game / 'src/main.rs').write_text(MAIN, encoding='utf-8', newline='\n')
    workspace.write_text(newline='\n', data=original.replace('members = [\n', 'members = [\n    "crates/native-game",\n', 1),
                         encoding='utf-8')
    return stage


def source_digest(stage):
    """Bind the whole staged source, including build scripts and embedded assets."""
    if not stage.is_dir() or stage.is_symlink():
        raise ValueError('staged source directory is missing or linked')
    def fail(error):
        raise error
    digest = hashlib.sha256()
    for folder, dirs, names in os.walk(stage, onerror=fail):
        excluded = ('.git', 'target') if Path(folder) == stage else ('.git',)
        dirs[:] = sorted(name for name in dirs if name not in excluded)
        for name in dirs + sorted(names):
            path = Path(folder) / name
            if path.is_symlink():
                raise ValueError('symlink in staged build inputs')
        for name in sorted(names):
            path = Path(folder) / name
            digest.update(path.relative_to(stage).as_posix().encode() + b'\0')
            with path.open('rb') as stream:
                digest.update(hashlib.file_digest(stream, 'sha256').digest())
    return digest.hexdigest()


def link(prepared, target):
    prepared, target = prepared.resolve(), target.resolve()
    if not prepared.is_relative_to((ROOT / '.local/clean-port').resolve()):
        raise ValueError('link requires an isolated clean-port workspace')
    if not target.is_relative_to((ROOT / '.local').resolve()) or target.is_relative_to(prepared):
        raise ValueError('target must be a separate repository-local cache')
    receipt = json.loads((prepared / 'game-entry-check.json').read_text(encoding='utf-8'))
    if receipt.get('exit') != 0 or receipt.get('horizon_revision') != PIN:
        raise ValueError('successful pinned game-entry check required')
    stage = prepared / 'applied'
    for relative, expected in [('src/main.rs', MAIN), ('Cargo.toml', MANIFEST)]:
        if (stage / 'crates/native-game' / relative).read_bytes() != expected.encode():
            raise ValueError('game-entry projection changed since typecheck')
    consumer = json.loads((prepared / 'consumer-check.json').read_text(encoding='utf-8'))
    if consumer['consumer_inputs'] != {name: hashlib.sha256(data).hexdigest()
                                     for name, data in checked_files(ROOT)[0].items()}:
        raise ValueError('consumer inputs changed since typecheck')
    inputs = source_digest(stage)
    if receipt.get('source_sha256') != inputs:
        raise ValueError('staged inputs changed or lack a snapshot digest; run a fresh entry check')
    output = prepared / 'game-entry-link.json'
    if output.exists():
        raise ValueError('link evidence already exists; preserve it')
    command = ['cargo', '+1.95.0', 'build', '--offline', '--locked', '--manifest-path',
               str(stage / 'Cargo.toml'), '--target-dir', str(target), '-p', PACKAGE,
               '--bin', 'horizon-client', '--message-format=json-render-diagnostics']
    env = dict(os.environ, CARGO_INCREMENTAL='0')
    result = subprocess.run(command, capture_output=True, text=True, env=env)
    (prepared / 'game-entry-link.log').write_text(newline='\n', data=result.stdout + '\n' + result.stderr, encoding='utf-8')
    proof = {'exit': result.returncode, 'command': command, 'horizon_revision': PIN,
             'source_sha256': inputs, 'binary_linked': False,
             'official_launcher_executed': False, 'gameplay_checked': False,
             'profile_environment': {key: value for key, value in env.items()
                 if key.startswith('CARGO_PROFILE_') or key in ('CARGO_INCREMENTAL', 'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS')}}
    if result.returncode == 0:
        try:
            if source_digest(stage) != inputs:
                raise ValueError('staged inputs changed during linking')
            artifacts = []
            for line in result.stdout.splitlines():
                message = json.loads(line)
                if (message.get('reason') == 'compiler-artifact'
                        and message.get('target', {}).get('name') == 'horizon-client'
                        and 'bin' in message['target'].get('kind', [])
                        and message.get('executable')):
                    artifacts.append(Path(message['executable']).resolve())
            if len(artifacts) != 1 or not artifacts[0].is_relative_to(target):
                raise ValueError('Cargo did not identify exactly one target-local game executable')
            with artifacts[0].open('rb') as stream:
                binary_hash = hashlib.file_digest(stream, 'sha256').hexdigest()
            proof.update(binary_linked=True, executable=str(artifacts[0]), binary_sha256=binary_hash)
        except (ValueError, OSError) as error:
            proof.update(exit=1, error=str(error))
    with output.open('x', encoding='utf-8') as stream:
        stream.write(json.dumps(proof, indent=2) + '\n')
    print(json.dumps(proof, indent=2))
    return proof['exit']


def check(prepared):
    stage = prepare(prepared)
    command = ['cargo', '+1.95.0', 'check', '--offline', '--manifest-path',
               str(stage / 'Cargo.toml'), '-p', PACKAGE, '--bin', 'horizon-client']
    code = subprocess.run(command).returncode
    inputs = None
    if code == 0:
        inputs = source_digest(stage)
        code = subprocess.run(command[:4] + ['--locked'] + command[4:]).returncode
        if source_digest(stage) != inputs:
            code = 1
    (prepared / 'game-entry-check.json').write_text(newline='\n', data=json.dumps({
        'exit': code, 'horizon_revision': PIN, 'source_sha256': inputs,
        'entry_sha256': hashlib.sha256(MAIN.encode()).hexdigest(),
        'manifest_sha256': hashlib.sha256(MANIFEST.encode()).hexdigest(),
        'official_launcher_executed': False, 'binary_linked': False, 'gameplay_checked': False,
    }, indent=2) + '\n', encoding='utf-8')
    return code


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prepared', type=Path, required=True)
    parser.add_argument('--link', action='store_true', help='Link a checked snapshot without launching or approving it')
    parser.add_argument('--target-dir', type=Path, default=ROOT / '.local/cargo-target-upstream-head')
    args = parser.parse_args()
    try:
        code = link(args.prepared, args.target_dir) if args.link else check(args.prepared.resolve())
    except (ValueError, OSError, KeyError) as error:
        parser.exit(1, f'Game-entry validation failed: {error}\n')
    raise SystemExit(code)
