#!/usr/bin/env python3
"""Exercise registered DATA markers and the production IslandPiece in a real game.

Explicit fixed-anchor pieces only. Runtime island palettes still omit the custom
fluid; natural starts, full island parity and saved-start reloads are not tested.
Run structures.py separately for the plain/native placement regression.
"""
import argparse
from collections import Counter
import gzip
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time

from native_build_report import require_report
from structures import ROOT, nbt_payload, prepare_resources


def wide_template():
    # Physical blocks in BOTH chunks ensure both native placement calls run.
    palette = [{'Name': (8, 'minecraft:gold_block')},
               {'Name': (8, 'minecraft:structure_block'),
                'Properties': (10, {'mode': (8, 'data')})}]
    blocks = []
    for x in (1, 17):
        blocks.append({'pos': (9, (3, [x, 0, 0])), 'state': (3, 0)})
        blocks.append({'pos': (9, (3, [x, 1, 0])), 'state': (3, 1),
                       'nbt': (10, {'mode': (8, 'DATA'), 'metadata': (8, 'probe_marker')})})
    root = {'size': (9, (3, [20, 2, 2])), 'palette': (9, (10, palette)),
            'blocks': (9, (10, blocks)), 'entities': (9, (10, []))}
    return gzip.compress(b'\x0a\x00\x00' + nbt_payload(10, root), mtime=0)


def prepare(original, run):
    resources, _ = prepare_resources(original, run)
    cases = ['marker_wide', 'marker_transform', 'marker_failure', 'marker_bad_state',
             'island_small', 'island_tall']
    settings = json.loads((ROOT / 'tconstruct/content/data/tconstruct/worldgen/structure/end_slime_island.json').read_text())
    bundle = resources / 'worldgen/structures.json'  # already detached by prepare_resources
    definitions = json.loads(bundle.read_text())
    added = []
    for index, case in enumerate(cases):
        x, y = index * 96, 140 if case.startswith('island_') else 110
        config = {'template': 'pluto_probe:marker_wide', 'origin': [x, y, 0],
                  'rotation': 'NONE', 'mirror': 'NONE', 'case': case}
        kind = 'pluto_probe:marker_piece'
        if case == 'marker_transform':
            config.update(rotation='CLOCKWISE_90', mirror='FRONT_BACK')
        elif case in ('marker_failure', 'marker_bad_state'):
            config.update(template='pluto_probe:marker', case='error' if case == 'marker_failure' else case)
            if case == 'marker_bad_state':
                config['initial_seen'] = -1
        elif case.startswith('island_'):
            kind = 'pluto_probe:island_piece'
            config.update(template='tconstruct:islands/ender/' + ('0x1x0' if case == 'island_small' else '11x1x11'),
                          tree='tconstruct:ender_slime_tree' + ('_tall' if case == 'island_tall' else ''),
                          grasses=settings['grasses'], vines=settings['vines'], trees_placed=0)
            if case == 'island_tall':
                config.update(rotation='CLOCKWISE_90', mirror='FRONT_BACK')
        pieces = [{'kind': kind, 'configuration': config}]
        if case == 'marker_bad_state':
            # A later invalid registered state must reject before this valid
            # earlier plain piece writes anything.
            pieces.insert(0, {'kind': 'pluto_lib:template_piece', 'configuration': {
                'template': 'pluto_probe:colors', 'origin': [x, y, 0], 'rotation': 'NONE', 'mirror': 'NONE'}})
        name = 'pluto_probe:' + case
        if name in definitions:
            raise RuntimeError('fixture structure already exists: ' + name)
        definition = {'type': 'pluto_probe:template_structure',
            'biomes': '#minecraft:is_overworld', 'step': 'surface_structures', 'spawn_overrides': {},
            'probe_case': case, 'probe_pieces': pieces}
        definitions[name] = json.dumps(definition)
        added.append(name)
        if case.startswith('island_'):
            # Tree markers may legitimately decline or fail placement. Retry
            # only on fresh, disjoint anchors; never change production RNG.
            for attempt in (1, 2):
                retry_name = name + f'_retry{attempt}'
                if retry_name in definitions:
                    raise RuntimeError('retry fixture already exists: ' + retry_name)
                retry = json.loads(json.dumps(definition))
                retry['probe_case'] = case + f'_retry{attempt}'
                retry['probe_pieces'][0]['configuration']['origin'][0] += attempt * 192
                definitions[retry_name] = json.dumps(retry)
                added.append(retry_name)
    bundle.write_text(json.dumps(definitions) + '\n', encoding='utf-8', newline='\n')
    commands = resources / 'command-registries.json'
    registry = json.loads(commands.read_text())
    ids = registry['by_registry']['minecraft:worldgen/structure']['ids']
    for name in added:
        if name in ids:
            raise RuntimeError('fixture command already exists: ' + name)
        ids.append(name)
    commands.write_text(json.dumps(registry) + '\n', encoding='utf-8', newline='\n')
    with (resources / 'worldgen/structure/pluto_probe/marker_wide.nbt').open('xb') as output:
        output.write(wide_template())
    # Do not patch feature catalogs to make the test pass: the production
    # resource preparation must have installed the named configured features.
    catalog = json.loads((resources / 'worldgen/features.json').read_text())
    for name in ('ender_slime_tree', 'ender_slime_tree_tall'):
        expected = json.loads((ROOT / f'tconstruct/content/data/tconstruct/worldgen/feature/{name}.json').read_text())
        actual = catalog.get('tconstruct:' + name)
        if isinstance(actual, str):
            actual = json.loads(actual)
        if actual != expected:
            raise RuntimeError('prepared configured feature differs: ' + name)
    return resources, cases


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ('game', 'ctl', 'resources'):
        parser.add_argument('--' + key, type=Path, required=True)
    args = parser.parse_args()
    game, controller, original = args.game.resolve(), args.ctl.resolve(), args.resources.resolve()
    pins = json.loads((ROOT / 'tconstruct/compatibility.json').read_text())
    require_report(game.parent, pins['horizon_revision'], pins['mixins_revision'],
                   {'pluto_lib', 'pluto_probe', 'tconstruct', 'pluto_native_creative'})
    run = Path(tempfile.mkdtemp(prefix='structure-markers-', dir=ROOT / '.local/smoke-runs')).resolve()
    resources, cases = prepare(original, run)
    endpoint, log = run / 'control.json', run / 'game.log'
    env = dict(os.environ, HORIZON_NO_AUDIO='1', HORIZON_NO_DISCORD='1', HORIZON_TELEMETRY_URL='off',
               HORIZON_LAUNCHER_NO_SELF_UPDATE='1', RUST_MIN_STACK='16777216', PLUTO_PROBE_STRUCTURE_MODE='1')
    for key in ('PLUTO_PROBE_FEATURE_MODE', 'PLUTO_PROBE_RESOURCE_PACK'):
        env.pop(key, None)
    for key in ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'TMP', 'TEMP'):
        folder = run / key.lower()
        folder.mkdir()
        env[key] = str(folder)
    observations, screenshots, attempted_origins = [], [], []

    def ctl(*words, timeout=40):
        words = list(map(str, words))
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

    def photo(case, camera, target):
        ctl('command', 'tp @s ' + ' '.join(map(str, camera)))
        ctl('look-at', *target)
        ctl('wait', 40)
        ctl('key', 'f1', 'tap')
        try:
            path = run / (case + '.png')
            ctl('screenshot', path)
            if not path.is_file() or not path.stat().st_size:
                raise RuntimeError('missing screenshot: ' + str(path))
            screenshots.append(str(path))
        finally:
            ctl('key', 'f1', 'tap')

    with log.open('w', encoding='utf-8', newline='\n') as output:
        proc = subprocess.Popen([str(game), '--resources', str(resources), '--control-socket', str(endpoint),
            '--offscreen', '--world', 'StructureMarkers', '--seed', '42', '--creative',
            '--render-distance', '4', '--width', '1280', '--height', '720'], cwd=run, env=env,
            stdout=output, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic() + 120
            while not endpoint.exists():
                if proc.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError(f'game did not start: {log}')
                time.sleep(.2)
            ctl('wait-ready', 120, timeout=125)
            for command in ('gamemode spectator', 'clear @s', 'time set 6000', 'weather clear', 'gamerule random_tick_speed 0'):
                ctl('command', command)
            for index, case in enumerate(cases):
                x, y = index * 96, 140 if case.startswith('island_') else 110
                ctl('command', f'tp @s {x+.5} {y+5} 8.5')
                ctl('wait', 60)
                before = ctl('blocks', x-3, y, -22, x+22, y+3, 3) if not case.startswith('island_') else None
                reply = ctl('command', f'place structure pluto_probe:{case} {x} {y} 0')
                ctl('wait', 20)
                if case in ('marker_wide', 'marker_transform'):
                    # Literal mirror FRONT_BACK then CLOCKWISE_90 transforms:
                    # source (1,0)->(0,-1); (17,0)->(0,-17).
                    points = [(x+1, 0), (x+17, 0)] if case == 'marker_wide' else [(x, -1), (x, -17)]
                    states = [ctl('getblock', px, y+1, pz)['block'] for px, pz in points]
                    if any(state != 'minecraft:oak_log[axis=x]' for state in states):
                        raise RuntimeError(f'{case}: marker transform/write failed: {states}; run={run}')
                    for px, pz in points:
                        if ctl('getblock', px, y, pz)['block'] != 'minecraft:gold_block':
                            raise RuntimeError(f'{case}: missing base block; run={run}')
                    observations.append({'case': case, 'states': states, 'reply': reply})
                    if case == 'marker_wide':
                        photo(case, (x+9, y+7, 20), (x+9, y+.5, 0))
                    else:
                        photo(case, (x+20, y+7, -9), (x, y+.5, -9))
                elif case == 'marker_failure':
                    if ctl('getblock', x, y, 0)['block'] != 'minecraft:gold_block':
                        raise RuntimeError(f'callback error lost earlier native template writes; run={run}')
                    if ctl('getblock', x+1, y+1, 1)['block'] != 'minecraft:air':
                        raise RuntimeError(f'failed callback unexpectedly painted marker; run={run}')
                    if any('Generated structure' in line for line in reply.get('chat', [])):
                        raise RuntimeError(f'callback error was reported as success; run={run}')
                    observations.append({'case': case, 'partial_writes_retained': True, 'reply': reply})
                    photo(case, (x+6, y+5, 9), (x+1, y+.5, .5))
                elif case == 'marker_bad_state':
                    after = ctl('blocks', x-3, y, -22, x+22, y+3, 3)
                    if before['blocks'] != after['blocks']:
                        raise RuntimeError(f'invalid later state allowed earlier piece writes; run={run}')
                    observations.append({'case': case, 'unchanged': True, 'reply': reply})
                    photo(case, (x+6, y+5, 9), (x+1, y+.5, .5))
                else:
                    attempts, base_x = [], x
                    for attempt in range(3):
                        x = base_x + attempt * 192
                        if attempt:
                            ctl('command', f'tp @s {x+.5} {y+5} 8.5')
                            ctl('wait', 60)
                            reply = ctl('command', f'place structure pluto_probe:{case}_retry{attempt} {x} {y} 0')
                            ctl('wait', 20)
                        attempted_origins.append(x)
                        counts = Counter()
                        for bottom in range(124, 190, 3):
                            counts.update(ctl('blocks', x-48, bottom, -48, x+48, bottom+2, 48)['counts'])
                        rows = re.findall(r'PLUTO-PROBE island_marker marker=(\S+) pos=\[(-?\d+), (-?\d+), (-?\d+)\] before=(\d+) after=(\d+) ok=(true|false)',
                                          log.read_text(encoding='utf-8', errors='replace'))
                        rows = [row for row in rows if x-32 <= int(row[1]) <= x+32]
                        if not rows or any(row[6] != 'true' for row in rows):
                            raise RuntimeError(f'{case}: missing markers or callback error; run={run}')
                        successful = max(int(row[5]) for row in rows)
                        attempts.append({'origin_x': x, 'trees_placed': successful,
                                         'region_counts': dict(counts), 'reply': reply})
                        if successful:
                            break
                    else:
                        raise RuntimeError(f'{case}: no successful tree after three fresh placements; run={run}')
                    for block in ('tconstruct:enderbark_log', 'tconstruct:ender_slime_leaves', 'tconstruct:ender_slime_vine'):
                        if not any(name.split('[', 1)[0] == block and count > 0 for name, count in counts.items()):
                            raise RuntimeError(f'{case}: no live {block} in observed region; run={run}')
                    if not any(name.split('[', 1)[0] in ('tconstruct:ender_slime_tall_grass', 'tconstruct:ender_slime_fern')
                               and count > 0 for name, count in counts.items()):
                        raise RuntimeError(f'{case}: no surviving marker grass; run={run}')
                    observations.append({'case': case, 'attempts': attempts,
                                         'fluid_parity': 'not implemented; existing structure_void palette retained'})
                    if case == 'island_small':
                        photo(case, (x+28, 166, 28), (x+10, 148, 10))
                    else:
                        photo(case, (x+4, 170, 4), (x-15, 150, -15))
            ctl('quit')
            proc.wait(timeout=60)
            if proc.returncode:
                raise RuntimeError(f'game exited {proc.returncode}: {log}')
        finally:
            if proc.poll() is None:
                proc.terminate()
                try:
                    proc.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait(timeout=15)
    text = log.read_text(encoding='utf-8', errors='replace')
    for case in ('marker_wide', 'marker_transform'):
        transitions = re.findall(rf'PLUTO-PROBE marker case={case} pos=.*? before=(\d+) after=(\d+) roll=', text)
        if sorted(transitions) != [('0', '1'), ('1', '2')]:
            raise RuntimeError(f'{case}: cross-chunk state missing/duplicated: {transitions}; run={run}')
    for expected in ('probe deliberately failed marker callback', 'probe rejected saved marker state'):
        if expected not in text:
            raise RuntimeError(f'expected rejection absent: {expected}; run={run}')
    island = re.findall(r'PLUTO-PROBE island_marker marker=(\S+) pos=\[(-?\d+), (-?\d+), (-?\d+)\] before=(\d+) after=(\d+) ok=(true|false)', text)
    if not island or any(ok != 'true' or not 0 <= int(before) <= int(after) <= 3 for _, _, _, _, before, after, ok in island):
        raise RuntimeError(f'island marker error/invalid count; run={run}')
    for origin in attempted_origins:
        rows = [row for row in island if origin-32 <= int(row[1]) <= origin+32]
        previous = 0
        for marker, _, _, _, before, after, _ in rows:
            before, after = int(before), int(after)
            if before != previous or after - before not in (0, 1) or (after != before and marker != 'tconstruct:slime_tree'):
                raise RuntimeError(f'island counter reset or changed outside a tree marker; run={run}')
            previous = after
        if not rows:
            raise RuntimeError(f'no production markers in attempted island at {origin}; run={run}')
    for phase in ('starting', 'stopping', 'stopped'):
        if text.count(f'PLUTO-PROBE server_{phase} dedicated=false') != 1:
            raise RuntimeError(f'unclean lifecycle: {phase}; run={run}')
    result = {'result': 'STRUCTURE MARKERS PASS', 'run': str(run), 'observations': observations,
              'screenshots': screenshots, 'scope': 'explicit fixed-anchor pieces; no custom fluid, natural starts or saved-start reload proof'}
    (run / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8', newline='\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
