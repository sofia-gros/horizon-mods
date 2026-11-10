#!/usr/bin/env python3
"""Real-game feature-type gate: column/codec/recursion, or native false/error ownership.

Requires the independent probe consumer. Resource fixtures are added only to an
isolated clone; existing input files are never changed. Screenshots need review.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time

from native_build_report import require_report

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--game', type=Path, required=True)
    parser.add_argument('--ctl', type=Path, required=True)
    parser.add_argument('--resources', type=Path, required=True)
    parser.add_argument('--mode', choices=('column', 'decline', 'invalid', 'flags'), default='column')
    args = parser.parse_args()
    game, controller, resources = args.game.resolve(), args.ctl.resolve(), args.resources.resolve()
    if not game.is_file() or not controller.is_file() or not resources.is_dir():
        parser.error('--game/--ctl must be files; --resources must be a prepared directory')
    pins = json.loads((ROOT / 'tconstruct/compatibility.json').read_text(encoding='utf-8'))
    require_report(game.parent, pins['horizon_revision'], pins['mixins_revision'], {'pluto_lib', 'pluto_probe', 'tconstruct'})
    runs = ROOT / '.local/smoke-runs'
    runs.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix=f'feature-{args.mode}-', dir=runs)).resolve()
    if args.mode in ('column', 'flags'):
        # Original files stay immutable; the clone's catalog is detached below.
        # Falling back to copying supports source folders on another filesystem.
        def link_or_copy(source, target):
            try:
                os.link(source, target)
            except OSError:
                if Path(target).exists():
                    raise RuntimeError(f'refusing to overwrite a linked resource: {target}')
                if shutil.disk_usage(run).free - Path(source).stat().st_size < 10 * 1024**3:
                    raise RuntimeError('resource copying would leave less than 10 GB free')
                shutil.copy2(source, target)
        original = resources
        resources = run / 'resources'
        shutil.copytree(original, resources, copy_function=link_or_copy)
        # Prepared resources load an id -> JSON-text bundle, not loose data files.
        # Break only this clone's hard link before replacing its fixture catalog.
        bundle = resources / 'worldgen/features.json'
        original_bytes = bundle.read_bytes()
        definitions = json.loads(original_bytes)
        fixtures = (
            ('column_valid', {'height': 3, 'state': 'minecraft:oak_log[axis=x]'}),
            ('column_bad', {'height': 0, 'state': 'minecraft:glass'}),
            ('column_recursive', {'height': 1, 'state': 'minecraft:glass', 'nested': 'pluto_probe:column_recursive'}),
        ) if args.mode == 'column' else tuple(
            (f'flags_{flags}', {'height': 1, 'state': 'minecraft:stone', 'flags': flags})
            for flags in (3, 2, 18, 19)
        )
        for name, config in fixtures:
            key = f'pluto_probe:{name}'
            if key in definitions:
                raise RuntimeError(f'refusing to overwrite an existing feature: {key}')
            definitions[key] = json.dumps({'type': 'pluto_probe:column', 'config': config})
        bundle.unlink()
        bundle.write_bytes((json.dumps(definitions) + '\n').encode('utf-8'))
        if (original / 'worldgen/features.json').read_bytes() != original_bytes:
            raise RuntimeError('the original feature catalog changed while preparing fixtures')
    endpoint, game_log = run / 'control.json', run / 'game.log'
    env = dict(os.environ, HORIZON_NO_AUDIO='1', HORIZON_NO_DISCORD='1', HORIZON_TELEMETRY_URL='off',
               HORIZON_LAUNCHER_NO_SELF_UPDATE='1', RUST_MIN_STACK='16777216', PLUTO_PROBE_FEATURE_MODE=args.mode)
    env.pop('PLUTO_PROBE_RESOURCE_PACK', None)
    for key in ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME'):
        folder = run / key.lower()
        folder.mkdir()
        env[key] = str(folder)
    temporary = run / 'tmp'
    temporary.mkdir()
    env['TMP'] = env['TEMP'] = str(temporary)
    screenshots, observations = [], []

    def ctl(*words, timeout=40):
        words = tuple(map(str, words))
        reply = subprocess.run([str(controller), '--socket', str(endpoint), '--timeout', str(timeout), *words],
                               capture_output=True, text=True, timeout=timeout + 5)
        with (run / 'ctl.jsonl').open('a', encoding='utf-8', newline='\n') as output:
            output.write(json.dumps({'args': words, 'code': reply.returncode, 'stdout': reply.stdout, 'stderr': reply.stderr}) + '\n')
        if reply.returncode:
            raise RuntimeError(f'control failed {words}: {reply.stdout} {reply.stderr}; run={run}')
        value = json.loads(reply.stdout)
        if not isinstance(value, dict) or value.get('error') or value.get('ok') is False:
            raise RuntimeError(f'control rejected {words}: {value}; run={run}')
        return value

    def block(x, y, z):
        return ctl('getblock', x, y, z)['block']

    def text():
        return game_log.read_text(encoding='utf-8', errors='replace')

    def fixture(x, ground, plant):
        for command in (f'fill {x - 3} 99 -3 {x + 3} 99 4 minecraft:stone',
                        f'fill {x - 3} 100 -3 {x + 3} 132 4 minecraft:air',
                        f'setblock {x} 100 0 {ground}', f'setblock {x} 101 0 {plant}',
                        f'setblock {x} 102 3 minecraft:stone', f'tp @s {x + 0.5} 103 3.5'):
            ctl('command', command)
        ctl('wait', 20)
        if block(x, 100, 0).split('[', 1)[0] != ground or block(x, 101, 0).split('[', 1)[0] != plant:
            raise RuntimeError(f'fixture did not survive: {x}/{plant}; run={run}')
        ctl('select', 'minecraft:bone_meal')

    def click_until(x, marker):
        for clicks in range(1, 33):
            ctl('look-at', 'block', x, 101, 0)
            ctl('interact', x, 101, 0, 'up')
            ctl('wait', 4)
            if marker in text():
                return clicks
        raise RuntimeError(f'expected callback not observed: {marker}; run={run}')

    def photograph(x, name):
        ctl('setblock', x, 102, 8, 'minecraft:stone')
        ctl('command', f'tp @s {x + 0.5} 103 8.5')
        ctl('setblock', x, 102, 3, 'minecraft:air')
        ctl('command', 'effect clear @s')
        ctl('look', 180, 0)
        ctl('wait', 40)
        ctl('key', 'f1', 'tap')
        try:
            path = run / name
            ctl('screenshot', path)
            if not path.is_file() or not path.stat().st_size:
                raise RuntimeError(f'screenshot missing: {path}')
            screenshots.append(str(path))
        finally:
            ctl('key', 'f1', 'tap')

    with game_log.open('w', encoding='utf-8', newline='\n') as output:
        proc = subprocess.Popen([str(game), '--resources', str(resources), '--control-socket', str(endpoint),
                                 '--offscreen', '--world', 'FeatureTypeProbe', '--seed', '42', '--creative',
                                 '--test-terrain', '--render-distance', '2', '--width', '1280', '--height', '720'],
                                cwd=run, env=env, stdout=output, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic() + 120
            while not endpoint.exists():
                if proc.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError(f'game did not start: {game_log}')
                time.sleep(.2)
            ctl('wait-ready', 120, timeout=125)
            for command in ('clear @s', 'time set 6000', 'weather clear', 'gamerule random_tick_speed 0', 'give @s minecraft:bone_meal 64'):
                ctl('command', command)
            if args.mode == 'column':
                for x, name, expected in ((-2, 'column_valid', True), (4, 'column_bad', False), (10, 'column_recursive', False)):
                    fixture(x, 'minecraft:stone', 'minecraft:dead_bush')
                    marker = f'PLUTO-PROBE feature_request name=pluto_probe:{name} placed={str(expected).lower()}'
                    clicks = click_until(x, marker)
                    states = [block(x, y, 0) for y in range(102, 106)]
                    if expected:
                        if any(not state.startswith('minecraft:oak_log[') or 'axis=x' not in state for state in states[:3]) or states[3] != 'minecraft:air':
                            raise RuntimeError(f'wrong generated column/properties: {states}; run={run}')
                        if 'PLUTO-PROBE custom_feature origin=[-2, 102, 0] height=3 rng_in_range=true' not in text():
                            raise RuntimeError(f'origin/configuration/RNG observation missing; run={run}')
                        photograph(x, 'custom-column.png')
                    elif states != ['minecraft:air'] * 4:
                        raise RuntimeError(f'failed codec/recursive feature wrote blocks: {states}; run={run}')
                    observations.append({'feature': name, 'clicks': clicks, 'states': states})
                observed = text()
                if observed.count('probe height must be 1..=8') != 1:
                    raise RuntimeError(f'expected only the bad fixture to fail decoding; run={run}')
                if 'Horizon: feature pluto_probe:' in observed:
                    raise RuntimeError(f'a custom request unexpectedly reached native feature decoding; run={run}')
            elif args.mode == 'flags':
                for x, flags in ((0, 3), (6, 2), (12, 18), (18, 19)):
                    fixture(x, 'minecraft:stone', 'minecraft:dead_bush')
                    ctl('setblock', x + 1, 102, 0, 'minecraft:diamond_block')
                    ctl('wait', 20)
                    name = f'pluto_probe:flags_{flags}'
                    clicks = click_until(x, f'PLUTO-PROBE feature_request name={name} placed=true')
                    ctl('wait', 10)
                    if block(x, 102, 0) != 'minecraft:stone' or block(x + 1, 102, 0) != 'minecraft:diamond_block':
                        raise RuntimeError(f'flagged write or observer state mismatch: {flags}; run={run}')
                    begin = f'PLUTO-PROBE flags_begin flags={flags} origin=[{x}, 102, 0]'
                    if text().count(begin) != 1:
                        raise RuntimeError(f'expected one flagged generation: {flags}; run={run}')
                    fragment = text().split(begin, 1)[1]
                    callbacks = fragment.count(f'PLUTO-PROBE flag_shape pos=[{x + 1}, 102, 0]')
                    known_shape = bool(flags & 16)
                    if (known_shape and callbacks != 0) or (not known_shape and callbacks == 0):
                        raise RuntimeError(f'wrong neighbour shape notification for flags={flags}: {callbacks}; run={run}')
                    observations.append({'flags': flags, 'clicks': clicks, 'shape_callbacks': callbacks})
                    photograph(x, f'flags-{flags}.png')
            else:
                fixture(0, 'tconstruct:ichor_slime_dirt', 'tconstruct:blood_slime_sapling')
                marker = 'PLUTO-PROBE native_feature declined' if args.mode == 'decline' else 'probe deliberately invalid native feature configuration'
                clicks = click_until(0, marker)
                states = [block(0, y, 0) for y in range(101, 133)]
                if not states[0].startswith('tconstruct:blood_slime_sapling') or states[1:] != ['minecraft:air'] * 31:
                    raise RuntimeError(f'registered failure fell through to native growth: {states}; run={run}')
                photograph(0, f'native-{args.mode}.png')
                observations.append({'mode': args.mode, 'clicks': clicks, 'native_growth_prevented': True})
            ctl('quit')
            proc.wait(timeout=60)
            if proc.returncode != 0:
                raise RuntimeError(f'game exited with {proc.returncode}: {game_log}')
        finally:
            if proc.poll() is None:
                proc.terminate()
                try:
                    proc.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait(timeout=15)
    lines = text().splitlines()
    lifecycle = [f'PLUTO-PROBE server_{phase} dedicated=false' for phase in ('starting', 'stopping', 'stopped')]
    if any(lines.count(event) != 1 for event in lifecycle) or [lines.index(event) for event in lifecycle] != sorted(lines.index(event) for event in lifecycle):
        raise RuntimeError(f'lifecycle must occur once and in order: {game_log}')
    print(json.dumps({'result': 'FEATURE TYPE PROBE PASS', 'mode': args.mode, 'run': str(run),
                      'observations': observations, 'screenshots': screenshots, 'image_review_required': True}, indent=2))


if __name__ == '__main__':
    main()
