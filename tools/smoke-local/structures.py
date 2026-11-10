#!/usr/bin/env python3
"""Place independent custom structures through the real native command path.

This checks explicit placement only, not natural starts or saved-start reloads.
Run against a matching full native probe build; inspect every saved screenshot.
"""
import argparse
import gzip
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import time

from native_build_report import require_report

ROOT = Path(__file__).resolve().parents[2]


def nbt_payload(kind, value):
    """Small writer for owned test fixtures, not a general untrusted NBT codec."""
    if kind == 3:
        return struct.pack('>i', value)
    if kind == 6:
        return struct.pack('>d', value)
    if kind == 8:
        raw = value.encode('ascii')
        return struct.pack('>H', len(raw)) + raw
    if kind == 9:
        element, values = value
        return bytes([element]) + struct.pack('>i', len(values)) + b''.join(nbt_payload(element, v) for v in values)
    if kind == 10:
        return b''.join(bytes([tag]) + nbt_payload(8, name) + nbt_payload(tag, data)
                        for name, (tag, data) in value.items()) + b'\0'
    raise ValueError(f'unsupported fixture NBT tag: {kind}')


def template(marker=False, entity=False):
    palette = [{'Name': (8, 'minecraft:' + name)} for name in
               ('gold_block', 'diamond_block', 'emerald_block', 'lapis_block', 'oak_stairs', 'air')]
    palette[4]['Properties'] = (10, {key: (8, value) for key, value in
                                   dict(facing='north', half='bottom', shape='straight', waterlogged='false').items()})
    positions = [(0, 0, 0), (2, 0, 0), (0, 0, 1), (2, 0, 1), (1, 1, 0), (1, 0, 0)]
    blocks = [{'pos': (9, (3, list(pos))), 'state': (3, index)} for index, pos in enumerate(positions)]
    if marker:
        palette.append({'Name': (8, 'minecraft:structure_block'), 'Properties': (10, {'mode': (8, 'data')})})
        blocks.append({'pos': (9, (3, [1, 1, 1])), 'state': (3, 6),
                       'nbt': (10, {'mode': (8, 'DATA'), 'metadata': (8, 'probe_marker')})})
    entities = [{'pos': (9, (6, [.5, 1., .5])), 'blockPos': (9, (3, [0, 1, 0])),
                 'nbt': (10, {'id': (8, 'minecraft:pig')})}] if entity else []
    root = {'size': (9, (3, [3, 2, 2])), 'palette': (9, (10, palette)),
            'blocks': (9, (10, blocks)), 'entities': (9, (10, entities))}
    return gzip.compress(b'\x0a\x00\x00' + nbt_payload(10, root), mtime=0)


def prepare_resources(original, run):
    def link_or_copy(source, destination):
        try:
            os.link(source, destination)
        except OSError:
            if Path(destination).exists():
                raise RuntimeError(f'refusing to overwrite linked resource: {destination}')
            if shutil.disk_usage(run).free - Path(source).stat().st_size < 10 * 1024**3:
                raise RuntimeError('resource copying would leave less than 10 GB free')
            shutil.copy2(source, destination)
    resources = run / 'resources'
    shutil.copytree(original, resources, copy_function=link_or_copy)
    bundle = resources / 'worldgen/structures.json'
    before = bundle.read_bytes()
    definitions = json.loads(before)
    # Rotation/mirror expectations below are literal independently worked points.
    cases = [('plain', 'NONE', 'NONE'), ('rotated', 'CLOCKWISE_90', 'NONE'),
             ('mirrored', 'NONE', 'LEFT_RIGHT'), ('combined', 'CLOCKWISE_90', 'FRONT_BACK'),
             ('reject', 'NONE', 'NONE'), ('error', 'NONE', 'NONE'), ('invalid', 'NONE', 'NONE'),
             ('bad_piece', 'NONE', 'NONE'), ('marker', 'NONE', 'NONE'), ('entity', 'NONE', 'NONE')]
    for index, (case, rotation, mirror) in enumerate(cases):
        x = index * 32
        name = 'pluto_probe:' + case
        if name in definitions:
            raise RuntimeError(f'refusing to overwrite structure: {name}')
        piece = {'kind': 'pluto_lib:template_piece', 'configuration': {
            'template': 'pluto_probe:' + (case if case in ('marker', 'entity') else 'colors'),
            'origin': [x, 110, 0], 'rotation': rotation, 'mirror': mirror}}
        pieces = [piece]
        if case == 'bad_piece':
            pieces.append({'kind': 'pluto_probe:unknown_piece', 'configuration': {}})
        definitions[name] = json.dumps({'type': 'pluto_probe:template_structure', 'biomes': '#minecraft:is_overworld',
                                       'step': 'surface_structures', 'spawn_overrides': {},
                                       'probe_case': case, 'probe_pieces': pieces})
    bundle.unlink()  # detach this clone, never overwrite the original hard link
    bundle.write_text(json.dumps(definitions) + '\n', encoding='utf-8', newline='\n')
    # Native commands validate against a separate prepared registry, not the
    # worldgen catalog. Match the content loader's by_registry additions.
    commands = resources / 'command-registries.json'
    commands_before = commands.read_bytes()
    registry = json.loads(commands_before)
    ids = registry['by_registry']['minecraft:worldgen/structure']['ids']
    for case, _, _ in cases:
        name = 'pluto_probe:' + case
        if name in ids:
            raise RuntimeError(f'refusing to overwrite command structure: {name}')
        ids.append(name)
    commands.unlink()
    commands.write_text(json.dumps(registry) + '\n', encoding='utf-8', newline='\n')
    if (original / 'command-registries.json').read_bytes() != commands_before:
        raise RuntimeError('original command registry changed during fixture preparation')
    for name in ('colors', 'marker', 'entity'):
        path = resources / f'worldgen/structure/pluto_probe/{name}.nbt'
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as output:
            output.write(template(marker=name == 'marker', entity=name == 'entity'))
    if (original / 'worldgen/structures.json').read_bytes() != before:
        raise RuntimeError('original structure catalog changed during fixture preparation')
    return resources, cases


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('game', 'ctl', 'resources'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    game, controller, original = args.game.resolve(), args.ctl.resolve(), args.resources.resolve()
    if not game.is_file() or not controller.is_file() or not original.is_dir():
        parser.error('game and ctl must be files; resources must be a prepared directory')
    pins = json.loads((ROOT / 'tconstruct/compatibility.json').read_text(encoding='utf-8'))
    require_report(game.parent, pins['horizon_revision'], pins['mixins_revision'],
                   {'pluto_lib', 'pluto_probe', 'tconstruct', 'pluto_native_creative'})
    runs = ROOT / '.local/smoke-runs'
    runs.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix='structures-', dir=runs)).resolve()
    resources, cases = prepare_resources(original, run)
    endpoint, game_log = run / 'control.json', run / 'game.log'
    env = dict(os.environ, HORIZON_NO_AUDIO='1', HORIZON_NO_DISCORD='1', HORIZON_TELEMETRY_URL='off',
               HORIZON_LAUNCHER_NO_SELF_UPDATE='1', RUST_MIN_STACK='16777216', PLUTO_PROBE_STRUCTURE_MODE='1')
    for key in ('PLUTO_PROBE_FEATURE_MODE', 'PLUTO_PROBE_RESOURCE_PACK'):
        env.pop(key, None)
    for key in ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'TMP', 'TEMP'):
        folder = run / key.lower()
        folder.mkdir()
        env[key] = str(folder)
    observations, screenshots = [], []

    def ctl(*words, timeout=40):
        words = tuple(map(str, words))
        reply = subprocess.run([str(controller), '--socket', str(endpoint), '--timeout', str(timeout), *words],
                               capture_output=True, text=True, timeout=timeout + 5)
        with (run / 'ctl.jsonl').open('a', encoding='utf-8', newline='\n') as output:
            output.write(json.dumps({'args': words, 'code': reply.returncode, 'stdout': reply.stdout, 'stderr': reply.stderr}) + '\n')
        if reply.returncode:
            raise RuntimeError(f'control failed {words}: {reply.stdout} {reply.stderr}; run={run}')
        result = json.loads(reply.stdout)
        if not isinstance(result, dict) or result.get('error') or result.get('ok') is False:
            raise RuntimeError(f'control rejected {words}: {result}; run={run}')
        return result

    def photograph(x, name, target_y=110.5, player_y=113):
        ctl('command', f'setblock {x+1} {player_y-1} 8 minecraft:stone')
        ctl('command', f'tp @s {x+1.5} {player_y} 8.5')
        ctl('command', 'effect clear @s')
        ctl('look-at', x + .5, target_y, .5)
        ctl('wait', 40)
        ctl('key', 'f1', 'tap')
        try:
            path = run / (name + '.png')
            ctl('screenshot', path)
            if not path.is_file() or not path.stat().st_size:
                raise RuntimeError(f'missing screenshot: {path}')
            screenshots.append(str(path))
        finally:
            ctl('key', 'f1', 'tap')

    # Coordinates for gold, diamond, emerald, lapis, stairs, explicit air.
    expected = [
        ([(0, 0), (2, 0), (0, 1), (2, 1), (1, 0), (1, 0)], 'north'),
        ([(0, 0), (0, 2), (-1, 0), (-1, 2), (0, 1), (0, 1)], 'east'),
        ([(0, 0), (2, 0), (0, -1), (2, -1), (1, 0), (1, 0)], 'south'),
        ([(0, 0), (0, -2), (-1, 0), (-1, -2), (0, -1), (0, -1)], 'east'),
    ]
    with game_log.open('w', encoding='utf-8', newline='\n') as output:
        proc = subprocess.Popen([str(game), '--resources', str(resources), '--control-socket', str(endpoint),
                                 '--offscreen', '--world', 'StructureProbe', '--seed', '42', '--creative',
                                 '--render-distance', '2', '--width', '1280', '--height', '720'],
                                cwd=run, env=env, stdout=output, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic() + 120
            while not endpoint.exists():
                if proc.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError(f'game did not start: {game_log}')
                time.sleep(.2)
            ctl('wait-ready', 120, timeout=125)
            for command in ('clear @s', 'time set 6000', 'weather clear', 'gamerule random_tick_speed 0'):
                ctl('command', command)
            for index, (case, _, _) in enumerate(cases):
                x = index * 32
                ctl('command', f'tp @s {x+.5} 113 8.5')
                ctl('wait', 60)
                ctl('command', f'fill {x-4} 109 -4 {x+4} 109 4 minecraft:stone')
                ctl('command', f'fill {x-4} 110 -4 {x+4} 114 4 minecraft:air')
                if index < 4:
                    dx, dz = expected[index][0][5]
                    ctl('command', f'setblock {x+dx} 110 {dz} minecraft:obsidian')
                before = ctl('blocks', x-4, 110, -4, x+4, 114, 4)
                reply = ctl('command', f'place structure pluto_probe:{case} {x} 110 0')
                ctl('wait', 20)
                after = ctl('blocks', x-4, 110, -4, x+4, 114, 4)
                if index < 4:
                    points, facing = expected[index]
                    names = ['gold_block', 'diamond_block', 'emerald_block', 'lapis_block', 'oak_stairs', 'air']
                    states = []
                    for item, ((dx, dz), name) in enumerate(zip(points, names)):
                        state = ctl('getblock', x+dx, 111 if item == 4 else 110, dz)['block']
                        if state.split('[', 1)[0] != 'minecraft:' + name:
                            raise RuntimeError(f'{case}: unexpected template block {item}: {state}; run={run}')
                        if item == 4 and not all(prop in state for prop in
                                                 (f'facing={facing}', 'half=bottom', 'shape=straight', 'waterlogged=false')):
                            raise RuntimeError(f'{case}: wrong transformed properties: {state}; run={run}')
                        states.append(state)
                    if sum(after['counts'].values()) != 5:
                        raise RuntimeError(f'{case}: unexpected extra/missing writes: {after}; run={run}')
                    observations.append({'case': case, 'states': states, 'reply': reply})
                else:
                    if before['blocks'] != after['blocks']:
                        raise RuntimeError(f'{case}: failed structure changed blocks; run={run}')
                    observations.append({'case': case, 'unchanged': True, 'reply': reply})
                photograph(x, case)
            # Built-in kinds must still dispatch natively even if their type has
            # a custom registry entry. Swamp huts require no external template.
            ctl('command', 'tp @s 352.5 100 8.5')
            ctl('wait', 60)
            native = ctl('command', 'place structure minecraft:swamp_hut 352 80 0')
            ctl('wait', 20)
            census = ctl('blocks', 348, 50, -4, 364, 100, 15)
            if not any(name.startswith('minecraft:spruce_planks') and count > 0 for name, count in census['counts'].items()):
                raise RuntimeError(f'native swamp hut not placed: {native}; run={run}')
            photograph(352, 'native-swamp-hut', target_y=70, player_y=80)
            observations.append({'case': 'native-swamp-hut', 'counts': census['counts'], 'reply': native})
            ctl('quit')
            proc.wait(timeout=60)
            if proc.returncode:
                raise RuntimeError(f'game exited {proc.returncode}: {game_log}')
        finally:
            if proc.poll() is None:
                proc.terminate()
                try:
                    proc.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait(timeout=15)
    text = game_log.read_text(encoding='utf-8', errors='replace')
    for case, _, _ in cases:
        if case != 'invalid' and f'PLUTO-PROBE structure case={case} ' not in text:
            raise RuntimeError(f'planner was not exercised: {case}; run={run}')
    for message in ('probe deliberately invalid structure configuration', 'probe deliberately failed structure planner',
                    'unsupported structure piece type', 'template marker/jigsaw dispatch is not connected',
                    'template entities are not supported by this piece codec'):
        if message not in text:
            raise RuntimeError(f'expected rejection was not observed: {message}; run={run}')
    if 'PLUTO-PROBE forbidden built-in structure override' in text:
        raise RuntimeError(f'custom codec intercepted native built-in kind; run={run}')
    for phase in ('starting', 'stopping', 'stopped'):
        if text.count(f'PLUTO-PROBE server_{phase} dedicated=false') != 1:
            raise RuntimeError(f'unclean lifecycle: {phase}; run={run}')
    result = {'result': 'STRUCTURES PASS', 'scope': 'explicit placement only; saved starts and natural generation unverified',
              'run': str(run), 'observations': observations, 'screenshots': screenshots}
    (run / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8', newline='\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
