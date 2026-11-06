#!/usr/bin/env python3
"""Prepared-only by default. Execute ONLY under the coordinator's serial grant.

Copies sealed applied v15 to a separate tmp overlay, appends one exact private
probe, checks the real gameplay lib, then replaces that addition with the exact
negative probe. Never edits official sources, the v15 stage or cache metadata.
"""
import argparse
import difflib
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shlex
import shutil
import signal
import subprocess
import time
import traceback

HERE = Path(__file__).resolve().parent
ROOT = Path('/workspace/scratch/c7b540ff3393')
CENTRAL = ROOT / 'upgrade-official-baseline-20261105'
BASE = Path('/tmp/pluto-v15-host-proof-20261106')
STAGE = BASE / 'staged'
PLAN = Path('/tmp/pluto-v15-plan-20261106')
LOGICAL = CENTRAL / '.work/plutolib-foundation-proof/current-staged'
TARGET = ROOT / '.work/official-advancement-recovery/host-target'
OFFICIAL = ROOT / '.devtools/horizon-latest-official-20261105'
SOURCE = 'crates/horizon-client/src/game.rs'
FLOOR = 10 * 1024**3 + 128 * 1024**2
TMP_FLOOR = 512 * 1024**2
PINS = {
    'stage-source-sha256.json': 'e16a3781aae1dea6d1a627f4bd28641469e16e822dae9704d0507a75a8d02428',
    'result.json': 'ccca310f860d9300d215dc0436e52f5be025dc5afa3304d363cacded03eab47b',
    'metadata.out': '34d7b2723634ef067a0d04a05308954e2d42dc41303ed0fa00669289131c3bce',
    'native-generated-inputs-used.json': 'ef6a6783b9581ca1e98b6f1efdea34f53d756551f8b48bbbd94316712bb1de88',
}
ARCHIVE = '7d9c4c5d7bd91ec6584b9206658f9d5bdb5d04dd3f95f1cfe90ceca6daae5386'
RUNTIME = 'f5a83539525c26fd2142e26e43789a3dd0107033'
PROBE_SHA = {
    'positive': '2199396795b3c26b94f618bbb243b00ed980705c45dce7028d807158a9518b1e',
    'negative': '5033febfe81224d4582f59a41bf470a35c19e5452c201d4314461b400032d01f',
}
spec = importlib.util.spec_from_file_location('prepare_game_borrow', HERE / 'prepare_game_borrow.py')
pristine = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pristine)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def sha(path):
    return digest(path.read_bytes())


def files(root):
    out = {}
    for path in sorted(root.rglob('*')):
        if path.is_symlink():
            raise ValueError('Symlink in sealed source: ' + str(path))
        if path.is_file():
            out[str(path.relative_to(root))] = sha(path)
    return out


def base_inputs():
    for name, expected in PINS.items():
        if sha(BASE / name) != expected:
            raise ValueError('Changed v15 evidence: ' + name)
    result = json.loads((BASE / 'result.json').read_text())
    if not result['full_type_proof'] or result['disk_guard_stopped']:
        raise ValueError('v15 is not a passed type gate')
    if result['runtime_source'] != RUNTIME or result['archive_sha256'] != ARCHIVE:
        raise ValueError('Different applied composition')
    if sha(PLAN / 'plutolib.hnmod') != ARCHIVE:
        raise ValueError('Different packaged composition')
    seal = json.loads((BASE / 'stage-source-sha256.json').read_text())
    if len(seal) != 5047 or files(STAGE) != seal:
        raise ValueError('Sealed v15 stage changed')
    times = json.loads((BASE / 'stage-source-mtime-ns.json').read_text())
    if set(times) != set(seal) or any((STAGE / n).stat().st_mtime_ns != t for n, t in times.items()):
        raise ValueError('v15 mtimes changed')
    # This is pristine source evidence, NOT a demand that applied v15 equal it.
    sources = pristine.read(OFFICIAL)
    for kind, expected in PROBE_SHA.items():
        if sha(HERE / f'game_borrow_{kind}.rs') != expected:
            raise ValueError('Probe changed: ' + kind)
    return seal, times, {n: digest(s.encode()) for n, s in sources.items()}


def proposed(original, kind):
    if kind not in PROBE_SHA:
        raise ValueError('Unknown probe')
    return original + b'\n' + (HERE / f'game_borrow_{kind}.rs').read_bytes()


def delta(original, kind):
    return ''.join(difflib.unified_diff(original.decode().splitlines(True),
        proposed(original, kind).decode().splitlines(True),
        fromfile='a/' + SOURCE, tofile='b/' + SOURCE))


def package_identity(metadata):
    # Same source/manifest/target identities and features as the proved v15 graph.
    return sorted((p['name'], p['id'], p['manifest_path'],
        tuple(sorted((t['name'], tuple(t['kind']), t['src_path']) for t in p['targets'])),
        json.dumps(p['features'], sort_keys=True)) for p in metadata['packages'])


def rustc_invocations(log):
    found = []
    for line in log.splitlines():
        if 'Running `' not in line or 'rustc' not in line:
            continue
        argv = shlex.split(line.split('Running `', 1)[1].rsplit('`', 1)[0])
        if '--crate-name' not in argv:
            continue
        found.append({'crate': argv[argv.index('--crate-name') + 1],
            'crate_type': argv[argv.index('--crate-type') + 1] if '--crate-type' in argv else None,
            'out_dirs': [x.split('=', 1)[1] for x in argv if x.startswith('OUT_DIR=')],
            'command': line})
    return found


def reject_diagnostic(messages, overlay_source):
    """Require exactly the prepared E0499, with primary span on the added call."""
    errors = []
    for line in messages.splitlines():
        obj = json.loads(line)  # Cargo JSON only; malformed/unrelated text fails closed.
        if obj.get('reason') == 'compiler-message' and obj['message']['level'] == 'error':
            errors.append(obj)
    if len(errors) != 1:
        raise ValueError('Expected exactly one compiler error')
    error = errors[0]
    msg = error['message']
    if (msg.get('code') or {}).get('code') != 'E0499':
        raise ValueError('Not the specified borrow rejection')
    if error['target']['name'] != 'horizon_client' or error['target']['kind'] != ['lib']:
        raise ValueError('Not the actual gameplay library')
    body = overlay_source.decode()
    marker = 'mod pluto_contact_negative_probe {'
    start = body.index(marker)
    call = body.index('        game.add_player_effect(effect);', start)
    line_number = body[:call].count('\n') + 1
    spans = [s for s in msg['spans'] if s['is_primary']]
    if len(spans) != 1:
        raise ValueError('Ambiguous primary diagnostic')
    span = spans[0]
    if not span['file_name'].replace('\\', '/').endswith(SOURCE):
        raise ValueError('Rejection outside the added Game source')
    if span['line_start'] != line_number or span['line_end'] != line_number:
        raise ValueError('Rejection at the wrong call')
    if not any('game.add_player_effect(effect);' in t['text'] for t in span['text']):
        raise ValueError('Missing exact rejected call')
    module_byte_start = len(body[:start].encode())
    if not (module_byte_start <= span['byte_start'] < span['byte_end'] <= len(overlay_source)):
        raise ValueError('Rejection outside the added module')
    return {'code': 'E0499', 'line': line_number, 'diagnostic': error}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--write-artifacts', action='store_true')
    parser.add_argument('--execute', action='store_true', help='Requires a separate coordinator grant')
    parser.add_argument('--output', type=Path, default=Path('/tmp/pluto-game-borrow-v15-20261106'))
    args = parser.parse_args()
    seal, times, official = base_inputs()
    original = (STAGE / SOURCE).read_bytes()
    for kind in PROBE_SHA:
        artifact = HERE / f'UNAPPLIED-v15-game-borrow-{kind}.patch'
        if args.write_artifacts:
            artifact.write_text(delta(original, kind))
        elif artifact.read_text() != delta(original, kind):
            raise ValueError('Applied-v15 addition diff changed: ' + kind)
    if not args.execute:
        print('Applied-v15 inputs and exact additions verified; no copy or compilation.')
        return
    output = args.output.absolute()
    if output.parent != Path('/tmp') or output.exists() or output.is_symlink():
        raise ValueError('Use a new direct /tmp evidence directory')
    if LOGICAL.exists() or LOGICAL.is_symlink():
        raise ValueError('Shared logical alias is occupied; do not replace it')
    result = {'status': 'incomplete', 'runtime_source': RUNTIME, 'package_sha256': ARCHIVE,
        'base_source_seal_sha256': PINS['stage-source-sha256.json'],
        'scope': 'Actual applied-v15 gameplay library compilation only; no actor/contact runtime proof',
        'minimum_free_bytes': shutil.disk_usage(ROOT).free,
        'minimum_tmp_free_bytes': shutil.disk_usage('/tmp').free,
        'disk_guard_stopped': False, 'phases': [], 'positive_pass': False,
        'negative_expected_rejection': False, 'runtime_executed': False}
    overlay = output / 'staged'
    alias_created = False
    expected_overlay = None
    input_hashes = files(HERE)

    def write(name, data):
        (output / name).write_text(json.dumps(data, indent=2) + '\n')

    def guard(extra_tmp=0):
        free = shutil.disk_usage(ROOT).free
        tmp = shutil.disk_usage('/tmp').free
        result['minimum_free_bytes'] = min(result['minimum_free_bytes'], free)
        result['minimum_tmp_free_bytes'] = min(result['minimum_tmp_free_bytes'], tmp)
        if free < FLOOR or tmp < TMP_FLOOR + extra_tmp:
            result['disk_guard_stopped'] = True
            raise RuntimeError('Capacity floor crossed')

    def check_sources():
        if base_inputs() != (seal, times, official):
            raise ValueError('Base or pristine evidence changed')
        if files(HERE) != input_hashes:
            raise ValueError('Probe sources changed during check')
        if expected_overlay is not None and files(overlay) != expected_overlay:
            raise ValueError('Overlay source/lock/file set changed during check')

    def run(name, command, expected_code=0):
        guard()
        prefix = f'set -e; source {ROOT}/.devtools/env.sh; '
        prefix += f'export C_INCLUDE_PATH={ROOT}/.devtools/sysroot/usr/include CARGO_TARGET_DIR={TARGET} '
        prefix += 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1; '
        start = time.monotonic()
        with (output / (name + '.out')).open('w') as out, (output / (name + '.err')).open('w') as err:
            child = subprocess.Popen(['bash', '-c', prefix + shlex.join(map(str, command))],
                cwd=CENTRAL, stdout=out, stderr=err, start_new_session=True)
            try:
                while child.poll() is None:
                    guard()
                    time.sleep(.1)
                code = child.wait()
            finally:
                if child.poll() is None:
                    os.killpg(child.pid, signal.SIGINT)
                    try:
                        child.wait(5)
                    except subprocess.TimeoutExpired:
                        os.killpg(child.pid, signal.SIGKILL)
                        child.wait()
                result['phases'].append({'name': name, 'returncode': child.returncode,
                    'seconds': time.monotonic() - start, 'command': list(map(str, command))})
                write('result.json', result)
        guard()
        if code != expected_code:
            raise RuntimeError(f'{name}: expected exit {expected_code}, got {code}')

    try:
        guard(sum((STAGE / n).stat().st_size for n in seal))
        output.mkdir()
        write('input-source-sha256.json', input_hashes)
        write('pristine-evidence-sha256.json', official)
        def guarded_copy(src, dst):
            guard()
            copied = shutil.copy2(src, dst)
            guard()
            return copied
        shutil.copytree(STAGE, overlay, copy_function=guarded_copy)
        if files(overlay) != seal:
            raise ValueError('Copy differs from sealed applied-v15 source')
        if any((overlay / n).stat().st_mtime_ns != ns for n, ns in times.items()):
            raise ValueError('Copy failed to preserve v15 source mtimes')
        # Exclusive symlink creation; never replace an existing alias.
        LOGICAL.symlink_to(overlay, target_is_directory=True)
        alias_created = True
        expected_overlay = seal.copy()
        run('metadata', ['cargo', 'metadata', '--manifest-path', LOGICAL / 'Cargo.toml',
            '--no-deps', '--format-version', '1', '--offline', '--locked'])
        new = json.loads((output / 'metadata.out').read_text())
        old = json.loads((BASE / 'metadata.out').read_text())
        if package_identity(new) != package_identity(old) or new['workspace_root'] != str(LOGICAL):
            raise ValueError('Cargo dependency/target identity differs from v15')
        check_sources()
        build_root = TARGET / 'debug/build'
        generated = {str(p): files(p) for p in sorted(build_root.glob('horizon-*/out')) if p.is_dir()}
        write('native-generated-inputs-before.json', generated)
        known = json.loads((BASE / 'native-generated-inputs-used.json').read_text())
        if any(generated.get(p) != h for p, h in known.items()):
            raise ValueError('Proved v15 native-generated inputs changed')
        for kind in ('positive', 'negative'):
            check_sources()
            replacement = proposed(original, kind)
            (overlay / SOURCE).write_bytes(replacement)
            expected_overlay = {**seal, SOURCE: digest(replacement)}
            write(kind + '-source-sha256.json', expected_overlay)
            run(kind, ['cargo', 'check', '--offline', '--locked', '--manifest-path', LOGICAL / 'Cargo.toml',
                '-p', 'horizon-client', '--lib', '--no-default-features', '--message-format=json', '-vv'],
                expected_code=0 if kind == 'positive' else 101)
            invocations = rustc_invocations((output / (kind + '.err')).read_text())
            host = [i for i in invocations if (i['crate'], i['crate_type']) == ('horizon_client', 'lib')]
            if len(host) != 1:
                raise ValueError('Real gameplay lib was not rechecked exactly once')
            used = {d for i in invocations for d in i['out_dirs'] if Path(d).name == 'out'}
            if not host[0]['out_dirs'] or any(d not in generated or files(Path(d)) != generated[d] for d in used):
                raise ValueError('Native-generated inputs drifted or were not observed')
            write(kind + '-rustc.json', invocations)
            write(kind + '-generated-inputs.json', {d: generated[d] for d in sorted(used)})
            check_sources()
            if kind == 'positive':
                result['positive_pass'] = True
            else:
                diagnostic = reject_diagnostic((output / 'negative.out').read_text(), replacement)
                write('negative-diagnostic.json', diagnostic)
                result['negative_expected_rejection'] = True
            write('result.json', result)
        check_sources()
        result['status'] = 'passed_compile_probes_only'
    except BaseException as error:
        result['error'] = str(error)
        if output.exists():
            (output / 'exception.log').write_text(traceback.format_exc())
    finally:
        if alias_created:
            if not LOGICAL.is_symlink() or os.readlink(LOGICAL) != str(overlay):
                result['status'] = 'incomplete'
                result['error'] = 'Alias owner changed; not removing another owner alias'
            else:
                LOGICAL.unlink()
                result['temporary_alias_removed'] = True
        try:
            guard()
            check_sources()
            result['base_hashes_and_mtimes_unchanged'] = True
            result['pristine_inputs_unchanged'] = True
            result['probe_inputs_unchanged'] = True
            result['overlay_sealed'] = expected_overlay is not None
        except BaseException as error:
            result['status'] = 'incomplete'
            result['final_verification_error'] = str(error)
        if output.exists():
            write('result.json', result)
        print(json.dumps(result, indent=2))
    raise SystemExit(0 if result['status'] == 'passed_compile_probes_only' else 1)


if __name__ == '__main__':
    main()
