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
    parser.add_argument('--mode', choices=('column', 'decline', 'invalid', 'flags', 'decorator', 'native_tree', 'roots', 'ordered', 'ender', 'propagules'), default='column')
    args = parser.parse_args()
    game, controller, resources = args.game.resolve(), args.ctl.resolve(), args.resources.resolve()
    if not game.is_file() or not controller.is_file() or not resources.is_dir():
        parser.error('--game/--ctl must be files; --resources must be a prepared directory')
    pins = json.loads((ROOT / 'tconstruct/compatibility.json').read_text(encoding='utf-8'))
    require_report(game.parent, pins['horizon_revision'], pins['mixins_revision'], {'pluto_lib', 'pluto_probe', 'tconstruct'})
    runs = ROOT / '.local/smoke-runs'
    runs.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix=f'feature-{args.mode}-', dir=runs)).resolve()
    if args.mode in ('column', 'flags', 'decorator', 'native_tree', 'roots', 'ordered', 'ender'):
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
        if args.mode == 'decorator':
            fixtures = (
                ('decorator_full', {'vines': 'minecraft:vine', 'probability': 1}),
                ('decorator_obstructed', {'vines': 'tconstruct:ender_slime_vine', 'probability': 1}),
                ('decorator_zero', {'vines': 'minecraft:vine', 'probability': 0}),
                ('decorator_invalid', {'vines': 'minecraft:vine', 'probability': 1.1}),
                ('decorator_write_fail', {'vines': 'pluto_probe:missing_vine', 'probability': 1}),
            )
        if args.mode == 'native_tree':
            oak = json.loads(definitions['minecraft:oak'])
            fixtures = (
                ('native_tree_valid', {'tree': oak}),
                ('native_tree_error', {'tree': oak, 'fail': True}),
                ('native_tree_invalid', {'tree': {'type': 'minecraft:no_op', 'config': {}}, 'remove': False}),
            )
        if args.mode == 'ender':
            reference = ROOT / 'tconstruct/content/reference/slime-islands/data/tconstruct/worldgen/configured_feature'
            sources = [json.loads((reference / name).read_text(encoding='utf-8')) for name in ('ender_slime_tree.json', 'ender_slime_tree_tall.json')]
            fixtures = tuple((f'ender_{index}', sources[index % 2]) for index in range(8))
        if args.mode == 'ordered':
            oak = json.loads(definitions['minecraft:oak'])
            oak.get('config', oak)['decorators'] = [
                {'type': 'minecraft:attached_to_logs', 'probability': 1,
                 'block_provider': {'type': 'minecraft:simple', 'state': {'id': block}}, 'directions': [direction]}
                for block, direction in [('minecraft:diamond_block', 'north'), ('minecraft:gold_block', 'south')]
            ]
            fixtures = tuple((f'ordered_{case}', {'tree': oak, 'case': case}) for case in ('valid', 'error', 'invalid'))
        if args.mode == 'roots':
            mangrove = json.loads(definitions['minecraft:mangrove'])
            config = mangrove.get('config', mangrove)
            config['decorators'] = []
            config['root_placer']['trunk_offset_y'] = 1
            placement = config['root_placer']['mangrove_root_placement']
            placement['can_grow_through'] = ['minecraft:water', 'minecraft:mud', 'minecraft:mangrove_roots', 'minecraft:muddy_mangrove_roots']
            placement['random_skew_chance'] = 0
            config['trunk_placer'].update(base_height=4, height_rand_a=0, height_rand_b=0, place_branch_per_log_probability=0)
            weighted = {'type': 'minecraft:weighted', 'entries': [
                {'weight': 1, 'data': {'id': 'minecraft:oak_log', 'properties': {'axis': 'x'}}},
                {'weight': 2, 'data': {'id': 'minecraft:birch_log', 'properties': {'axis': 'z'}}},
            ]}
            def simple(name, **properties):
                return {'type': 'minecraft:simple', 'state': {'id': name, 'properties': properties}}
            def variant(matches, provider):
                return {'matches': matches, 'state_provider': provider}
            diamond = simple('minecraft:diamond_block')
            cases = {
                'weighted': [variant(['minecraft:air'], weighted), variant(['minecraft:air'], diamond)],
                'water': [variant(['minecraft:water'], simple('minecraft:mangrove_roots', waterlogged='false')), variant(['minecraft:water'], diamond)],
                'parent': [variant(['minecraft:stone'], diamond)],
                'no_state': [variant(['minecraft:air'], {'type': 'minecraft:rule_based', 'rules': []}), variant(['minecraft:air'], diamond)],
                'invalid_binding': [variant(['minecraft:air'], weighted), variant(['minecraft:stone'], simple('pluto_probe:missing_root'))],
            }
            fixtures = tuple((f'roots_{case}', {'case': case, 'tree': mangrove, 'roots': {'root_variants': variants}}) for case, variants in cases.items())
        for name, config in fixtures:
            key = f'pluto_probe:{name}'
            if key in definitions:
                raise RuntimeError(f'refusing to overwrite an existing feature: {key}')
            kind = {'decorator': 'pluto_probe:decorator_fixture', 'native_tree': 'pluto_probe:native_tree_fixture', 'roots': 'pluto_probe:root_fixture', 'ordered': 'pluto_probe:ordered_fixture', 'ender': 'tconstruct:source_tree'}.get(args.mode, 'pluto_probe:column')
            definitions[key] = json.dumps({'type': kind, 'config': config})
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

    def fixture(x, ground, plant, radius=3, top=132):
        for command in (f'fill {x - radius} 99 {-radius} {x + radius} 99 {radius + 1} minecraft:stone',
                        f'fill {x - radius} 100 {-radius} {x + radius} {top} {radius + 1} minecraft:air',
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

    def photograph(x, name, player_y=103, camera_z=8):
        ctl('setblock', x, player_y - 1, camera_z, 'minecraft:stone')
        ctl('command', f'tp @s {x + 0.5} {player_y} {camera_z + 0.5}')
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
            elif args.mode == 'propagules':
                sapling = 'tconstruct:ender_slime_sapling'
                leaf = 'tconstruct:ender_slime_leaves'
                ctl('command', f'give @s {sapling} 8')

                def setup(x):
                    ctl('command', f'tp @s {x+0.5} 150 15.5')
                    ctl('wait', 40)
                    fixture(x, 'tconstruct:ender_slime_dirt', 'minecraft:air', radius=12, top=140)

                def use(x, y, small=False):
                    if small:
                        # aim_block samples only cube centre-lines. Hanging
                        # propagules have a thin XZ-offset outline; aim at a
                        # visible point and verify the real crosshair ray before
                        # a normal mouse-use action. No forced hit or occlusion
                        # override, and the server-state assertions stay intact.
                        for dx in (.5, .375, .625, .25, .75):
                            for dz in (.5, .375, .625, .25, .75):
                                ctl('look-at', x+dx, y+.94, dz)
                                ctl('wait', 1)
                                target = ctl('looking-at')
                                hit = target.get('block') or {}
                                if target.get('kind') == 'block' and hit.get('pos') == [x, y, 0] and hit.get('block') == sapling:
                                    ctl('use-item')
                                    ctl('wait', 8)
                                    return
                        raise RuntimeError(f'no visible propagule outline point: {x}/{y}; run={run}')
                    ctl('look-at-block', x, y, 0)
                    ctl('interact', x, y, 0)
                    ctl('wait', 8)

                def require_state(x, y, kind, **properties):
                    actual = block(x, y, 0)
                    if actual.split('[', 1)[0] != kind or any(f'{key}={value}' not in actual for key, value in properties.items()):
                        raise RuntimeError(f'propagule state mismatch: {x}/{y}: {actual}; run={run}')
                    return actual

                # Real bone meal on Ender leaves, then on the hanging sapling.
                setup(0)
                ctl('setblock', 0, 104, 0, leaf + '[persistent=true]')
                ctl('select', 'minecraft:bone_meal')
                use(0, 104)
                require_state(0, 103, sapling, age=0, hanging='true')
                # Age-0 geometry is tucked under the leaf. Look from below it,
                # rather than trying to click through the supporting leaf.
                ctl('setblock', 0, 102, 3, 'minecraft:air')
                ctl('setblock', 0, 100, 3, 'minecraft:stone')
                ctl('command', 'tp @s 0.5 101 3.5')
                ctl('wait', 20)
                for age in range(1, 5):
                    use(0, 103, small=True)
                    require_state(0, 103, sapling, age=age, hanging='true')
                mature = block(0, 103, 0)
                use(0, 103, small=True); use(0, 104)
                if block(0, 103, 0) != mature:
                    raise RuntimeError(f'mature/occupied target was changed; run={run}')
                photograph(0, 'propagule-mature.png', player_y=103, camera_z=5)
                ctl('setblock', 0, 104, 0, 'minecraft:air')
                ctl('wait', 20)
                require_state(0, 103, 'minecraft:air')
                observations.append({'case': 'leaf-bonemeal-maturation-support', 'ages': list(range(5)), 'removed_without_leaf': True})

                for x, water in ((48, False), (96, True)):
                    setup(x)
                    if water: ctl('setblock', x, 101, 0, 'minecraft:water')
                    ctl('select', sapling)
                    use(x, 100)
                    require_state(x, 101, sapling, age=4, stage=0, hanging='false', waterlogged=str(water).lower())
                    photograph(x, f'propagule-placement-{water}.png', player_y=102, camera_z=6)
                    observations.append({'case': 'placement', 'waterlogged': water, 'age': 4})
                    if water: continue
                    ctl('command', f'tp @s {x+0.5} 103 3.5')
                    ctl('setblock', x, 102, 3, 'minecraft:stone')
                    ctl('select', 'minecraft:bone_meal')
                    saw_stage = False
                    for attempt in range(32):
                        use(x, 101)
                        actual = block(x, 101, 0)
                        saw_stage |= actual.startswith(sapling + '[') and 'stage=1' in actual
                        if not actual.startswith(sapling + '['): break
                    logs = sum(block(x, y, 0).split('[', 1)[0] == 'tconstruct:enderbark_log' for y in range(102, 133))
                    if not saw_stage or not logs:
                        raise RuntimeError(f'ground sapling did not advance into an Ender tree; run={run}')
                    photograph(x, 'propagule-bonemeal-tree.png', player_y=109, camera_z=24)
                    observations.append({'case': 'bonemeal-growth', 'requests': attempt+1, 'stage_one_seen': saw_stage, 'logs': logs})

                setup(144)
                ctl('setblock', 144, 104, 0, leaf + '[persistent=true]')
                ctl('setblock', 144, 103, 0, sapling + '[hanging=true,age=0,stage=0,waterlogged=false]')
                ctl('command', 'gamerule random_tick_speed 4096')
                try:
                    for _ in range(24):
                        ctl('wait', 40)
                        if 'age=4' in block(144, 103, 0): break
                    require_state(144, 103, sapling, age=4, hanging='true')
                finally:
                    ctl('command', 'gamerule random_tick_speed 0')
                photograph(144, 'propagule-random-mature.png', player_y=103, camera_z=5)
                observations.append({'case': 'random-hanging-maturation', 'age': 4})

                setup(192)
                ctl('setblock', 192, 101, 0, sapling + '[hanging=false,age=4,stage=0,waterlogged=false]')
                ctl('command', 'gamerule random_tick_speed 4096')
                try:
                    for _ in range(24):
                        ctl('wait', 40)
                        if not block(192, 101, 0).startswith(sapling + '['): break
                finally:
                    ctl('command', 'gamerule random_tick_speed 0')
                logs = sum(block(192, y, 0).split('[', 1)[0] == 'tconstruct:enderbark_log' for y in range(102, 133))
                if not logs:
                    raise RuntimeError(f'random ground growth did not generate Ender logs; run={run}')
                photograph(192, 'propagule-random-tree.png', player_y=109, camera_z=24)
                observations.append({'case': 'random-ground-growth', 'logs': logs})
            elif args.mode == 'ender':
                import re
                total_propagules = 0
                for index in range(8):
                    x = index * 48
                    flavor = ('earth', 'sky', 'ichor', 'ender')[index // 2]
                    ctl('command', f'tp @s {x + 0.5} 150 15.5')
                    ctl('wait', 40)
                    # Split clearing so every fill stays below the native limit.
                    for command in (f'fill {x-16} 99 -16 {x+16} 99 16 minecraft:stone',
                                    f'fill {x-16} 100 -16 {x+16} 120 16 minecraft:air',
                                    f'fill {x-16} 121 -16 {x+16} 145 16 minecraft:air',
                                    f'fill {x-16} 100 -16 {x+16} 100 16 tconstruct:{flavor}_congealed_slime',
                                    f'setblock {x} 101 0 minecraft:dead_bush',
                                    f'setblock {x} 102 3 minecraft:stone', f'tp @s {x+0.5} 103 3.5'):
                        ctl('command', command)
                    ctl('wait', 20)
                    ctl('select', 'minecraft:bone_meal')
                    name = f'pluto_probe:ender_{index}'
                    clicks = click_until(x, f'PLUTO-PROBE feature_request name={name} placed=true')
                    match = re.search(rf'PLUTO-PROBE ender_case index={index} placed=true logs=(\d+) leaves=(\d+) vines=(\d+) propagules=(\d+) roots=(\d+) variants=(\d+) states_ok=true', text())
                    if not match:
                        raise RuntimeError(f'Ender tree observation missing: {index}; run={run}')
                    counts = dict(zip(('logs', 'leaves', 'vines', 'propagules', 'roots', 'variants'), map(int, match.groups())))
                    if any(counts[key] <= 0 for key in ('logs', 'leaves', 'vines', 'roots', 'variants')):
                        raise RuntimeError(f'incomplete Ender tree: {counts}; run={run}')
                    total_propagules += counts['propagules']
                    photograph(x, f'ender-{flavor}-{index%2}.png', player_y=109, camera_z=24)
                    observations.append({'case': index, 'flavor': flavor, 'tall': bool(index%2), 'clicks': clicks, **counts})
                if total_propagules == 0 or 'Horizon: feature pluto_probe:' in text():
                    raise RuntimeError(f'Ender propagules missing or native fallback used; run={run}')
            elif args.mode == 'ordered':
                for x, case, success, count in ((0, 'valid', True, 3), (16, 'error', False, 1), (32, 'invalid', False, 0)):
                    fixture(x, 'minecraft:dirt', 'minecraft:dead_bush')
                    name = f'pluto_probe:ordered_{case}'
                    clicks = click_until(x, f'PLUTO-PROBE feature_request name={name} placed={str(success).lower()}')
                    marker = f'PLUTO-PROBE ordered_case case={case} placed={str(success).lower()} callbacks={count} valid=true'
                    if marker not in text():
                        raise RuntimeError(f'ordered placement observation missing: {case}; run={run}')
                    if case == 'invalid' and not block(x, 101, 0).startswith('minecraft:dead_bush'):
                        raise RuntimeError(f'invalid plan wrote blocks; run={run}')
                    photograph(x, f'ordered-{case}.png')
                    observations.append({'case': case, 'clicks': clicks, 'placed': success, 'callbacks': count})
                if 'probe ordered decorator failure' not in text() or 'Horizon: feature pluto_probe:' in text():
                    raise RuntimeError(f'ordered failure ownership diagnostics missing; run={run}')
            elif args.mode == 'roots':
                import re
                for x, case, success in ((0, 'weighted', True), (40, 'water', True), (80, 'parent', True), (120, 'no_state', False), (160, 'invalid_binding', False)):
                    ctl('command', f'tp @s {x + 0.5} 150 15.5')
                    ctl('wait', 40)
                    fixture(x, 'minecraft:dirt', 'minecraft:dead_bush', radius=12, top=140)
                    name = f'pluto_probe:roots_{case}'
                    clicks = click_until(x, f'PLUTO-PROBE feature_request name={name} placed={str(success).lower()}')
                    observed = text()
                    match = re.search(r'PLUTO-PROBE root_case case=' + case + r' placed=(true|false) calls=(\d+) selected=(\d+) parent=(\d+) errors=(\d+) weighted_ok=(true|false) center=(\S+)', observed)
                    if not match or (match.group(1) == 'true') != success:
                        raise RuntimeError(f'root case observation missing: {case}; run={run}')
                    calls, selected, parent, errors = map(int, match.group(2, 3, 4, 5))
                    center = block(x, 101, 0)
                    if case == 'weighted' and (selected == 0 or match.group(6) != 'true' or center.split('[', 1)[0] not in ('minecraft:oak_log', 'minecraft:birch_log')):
                        raise RuntimeError(f'first weighted provider was not preserved: {center}; run={run}')
                    if case == 'water' and (not center.startswith('minecraft:mangrove_roots[') or 'waterlogged=true' not in center):
                        raise RuntimeError(f'custom root was not waterlogged: {center}; run={run}')
                    if case == 'parent' and (selected != 0 or parent == 0 or not center.startswith('minecraft:muddy_mangrove_roots[')):
                        raise RuntimeError(f'native muddy-root parent fallback missing: {center}; run={run}')
                    if case == 'no_state' and (calls != 1 or errors != 1 or center != 'minecraft:air'):
                        raise RuntimeError(f'selected-provider failure fell through: {center}; run={run}')
                    if case == 'invalid_binding' and (calls != 0 or not center.startswith('minecraft:dead_bush')):
                        raise RuntimeError(f'invalid unselected provider was not rejected before writes: {center}; run={run}')
                    observations.append({'case': case, 'clicks': clicks, 'placed': success, 'calls': calls, 'selected': selected, 'parent': parent, 'errors': errors, 'center': center})
                    photograph(x, f'roots-{case}.png', player_y=103, camera_z=20)
                observed = text()
                if 'PlutoLib: root binding failed:' not in observed or 'PlutoLib: tree placement failed:' not in observed:
                    raise RuntimeError(f'root binding/provider failure diagnostics missing; run={run}')
                if 'Horizon: feature pluto_probe:' in observed:
                    raise RuntimeError(f'root failure reached native feature fallback; run={run}')
            elif args.mode == 'native_tree':
                import re
                for x, case, success in ((0, 'valid', True), (16, 'error', False), (32, 'invalid', False)):
                    fixture(x, 'minecraft:dirt', 'minecraft:dead_bush')
                    ctl('setblock', x + 6, 101, 0, 'minecraft:air')
                    ctl('setblock', x + 7, 101, 0, 'minecraft:gold_block')
                    ctl('wait', 10)
                    name = f'pluto_probe:native_tree_{case}'
                    clicks = click_until(x, f'PLUTO-PROBE feature_request name={name} placed={str(success).lower()}')
                    ctl('wait', 10)
                    observed = text()
                    origin = f'[{x}, 101, 0]'
                    begin = f'PLUTO-PROBE tree_extra_begin origin={origin}'
                    if case == 'invalid':
                        if begin in observed or block(x + 6, 101, 0) != 'minecraft:air' or not block(x, 101, 0).startswith('minecraft:dead_bush'):
                            raise RuntimeError(f'invalid native tree wrote blocks or called decorators; run={run}')
                        callbacks = 0
                        counts = None
                    else:
                        if observed.count(begin) != 1 or block(x + 6, 101, 0) != 'minecraft:diamond_block':
                            raise RuntimeError(f'native tree callback/write missing; run={run}')
                        context = next((line for line in observed.splitlines() if f'PLUTO-PROBE native_tree_context origin={origin}' in line), '')
                        if 'rng_in_range=true nested_rejected=true' not in context:
                            raise RuntimeError(f'wrong tree context/RNG/nested-write handling: {context}; run={run}')
                        match = re.search(r'PLUTO-PROBE native_tree origin=' + re.escape(origin) + r' placed=(true|false) logs=(\d+) leaves=(\d+) vines=(\d+) distances_ok=(true|false)', observed)
                        if not match or min(map(int, match.group(2, 3, 4))) <= 0 or (success and match.group(5) != 'true'):
                            raise RuntimeError(f'tree logs/leaves/vines/final distances missing; run={run}')
                        counts = list(map(int, match.group(2, 3, 4)))
                        if any(not block(x, y, 0).startswith('minecraft:oak_log[') for y in range(101, 105)):
                            raise RuntimeError(f'native tree trunk missing; run={run}')
                        callbacks = observed.split(begin, 1)[1].count(f'PLUTO-PROBE tree_edge pos=[{x + 7}, 101, 0]')
                        if (success and callbacks == 0) or (not success and callbacks != 0):
                            raise RuntimeError(f'wrong tracked-write edge finalization: {case}/{callbacks}; run={run}')
                    observations.append({'case': case, 'clicks': clicks, 'placed': success, 'logs_leaves_vines': counts, 'edge_callbacks': callbacks})
                    photograph(x, f'native-tree-{case}.png', player_y=103, camera_z=16)
                observed = text()
                if 'probe deliberate tree decorator failure' not in observed or 'expected a tree feature' not in observed:
                    raise RuntimeError(f'native tree failure diagnostics missing; run={run}')
                if 'Horizon: feature pluto_probe:' in observed:
                    raise RuntimeError(f'native tree callback failure fell back; run={run}')
            elif args.mode == 'decorator':
                scenes = ((0, 'full', 'minecraft:vine', True),
                          (8, 'obstructed', 'tconstruct:ender_slime_vine', True),
                          (16, 'zero', 'minecraft:vine', True),
                          (24, 'invalid', None, False),
                          (32, 'write_fail', None, False))
                for x, case, vine, success in scenes:
                    fixture(x, 'minecraft:stone', 'minecraft:dead_bush')
                    if case == 'obstructed':
                        ctl('setblock', x, 107, -1, 'minecraft:stone')
                    name = f'pluto_probe:decorator_{case}'
                    marker = f'PLUTO-PROBE feature_request name={name} placed={str(success).lower()}'
                    clicks = click_until(x, marker)
                    ctl('wait', 10)
                    anchor = block(x, 108, 0)
                    if case == 'invalid':
                        if anchor != 'minecraft:air':
                            raise RuntimeError(f'invalid decorator wrote before decoding: {anchor}; run={run}')
                    elif not anchor.startswith('minecraft:oak_leaves['):
                        raise RuntimeError(f'decorator anchor missing: {case}; run={run}')
                    count = 0
                    for dx, dz, face in ((0, -1, 'south'), (1, 0, 'west'), (0, 1, 'north'), (-1, 0, 'east')):
                        for y in range(103, 109):
                            state = block(x + dx, y, dz)
                            expected_vine = case in ('full', 'obstructed') and y >= 104
                            if case == 'obstructed' and (dx, dz) == (0, -1):
                                if y == 107:
                                    if state != 'minecraft:stone':
                                        raise RuntimeError(f'vine overwrote obstruction: {state}; run={run}')
                                    continue
                                expected_vine = y == 108
                            if expected_vine:
                                if not state.startswith(vine + '['):
                                    raise RuntimeError(f'wrong vine at {(x + dx, y, dz)}: {state}; run={run}')
                                properties = dict(p.split('=') for p in state.split('[', 1)[1].rstrip(']').split(','))
                                if any(properties.get(side) != ('true' if side == face else 'false') for side in ('north', 'east', 'south', 'west')) or properties.get('up') != 'false':
                                    raise RuntimeError(f'wrong default-state attachment: {state}; run={run}')
                                count += 1
                            elif state != 'minecraft:air':
                                raise RuntimeError(f'unexpected decorator write at {(x + dx, y, dz)}: {state}; run={run}')
                    if count != (20 if case == 'full' else 16 if case == 'obstructed' else 0):
                        raise RuntimeError(f'wrong vine count: {case}/{count}; run={run}')
                    observations.append({'case': case, 'clicks': clicks, 'vines': count, 'placed': success})
                    photograph(x, f'decorator-{case}.png', player_y=106)
                observed = text()
                if observed.count('probability must be in 0..=1') != 1 or 'vine write failed:' not in observed:
                    raise RuntimeError(f'decorator validation/write diagnostics missing; run={run}')
                if 'Horizon: feature pluto_probe:' in observed:
                    raise RuntimeError(f'decorator failure reached native decoding; run={run}')
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
