#!/usr/bin/env python3
"""Exercise production slime soils and Earth/Sky saplings on an existing build.

Uses real item placement and bone meal, never substitute probe block callbacks.
Checks canopy geometry independently from Java's diamond-layer coordinates;
records leaf distances without claiming complete Java feature/RNG parity.
"""
import argparse
from collections import Counter
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

from native_build_report import require_report
from structures import ROOT


def canopy(origin, height):
    """SlimeTreeFeature.placeCanopy without vines, including its repeated drip."""
    x, y, z = origin
    logs = {(x, y + dy, z) for dy in range(height)}
    leaves = {(x + dx, y + height - i, z + dz)
              for i in range(4) for dx in range(-i - 1, i + 2)
              for dz in range(-i - 1, i + 2) if abs(dx) + abs(dz) <= i + 1} - logs
    holes = {(x + dx, y + height - 3, z + dz)
             for dx, dz in ((4, 0), (-4, 0), (0, 4), (0, -4))}
    leaves -= holes
    leaves.update((x + dx, y + height - 4, z + dz) for dx, dz in
                  ((3, 0), (-3, 0), (0, -3), (0, 3), (1, 1), (-3, 0), (-1, 1), (-1, -1)))
    return logs, leaves, holes


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('game', 'ctl', 'resources'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    game, controller, resources = (getattr(args, name).resolve() for name in ('game', 'ctl', 'resources'))
    pins = json.loads((ROOT / 'tconstruct/compatibility.json').read_text(encoding='utf-8'))
    require_report(game.parent, pins['horizon_revision'], pins['mixins_revision'],
                   {'pluto_lib', 'pluto_probe', 'tconstruct', 'pluto_native_creative'})
    run = Path(tempfile.mkdtemp(prefix='soil-trees-', dir=ROOT / '.local/smoke-runs')).resolve()
    endpoint = run / 'control.json'
    # Load the exact production tag through Horizon's content-mod loader, not
    # by editing exported survival tables or replacing plant callbacks.
    content = run / 'production-tag'
    tag_path = Path('data/minecraft/tags/block/supports_vegetation.json')
    (content / tag_path.parent).mkdir(parents=True)
    tag_bytes = (ROOT / 'tconstruct/content' / tag_path).read_bytes()
    (content / tag_path).write_bytes(tag_bytes)
    (content / 'horizon-mod.toml').write_bytes(
        b'id = "soil_support_probe"\nname = "Production soil support tag"\nversion = "0.1.0"\napi = "0.2"\ncapabilities = ["registries"]\n')
    env = dict(os.environ, HORIZON_NO_AUDIO='1', HORIZON_NO_DISCORD='1', HORIZON_TELEMETRY_URL='off',
               HORIZON_LAUNCHER_NO_SELF_UPDATE='1', RUST_MIN_STACK='16777216', PLUTO_PROBE_STRUCTURE_MODE='1')
    for key in ('PLUTO_PROBE_FEATURE_MODE', 'PLUTO_PROBE_NATURAL_MODE', 'PLUTO_PROBE_NATURAL_RESUME', 'PLUTO_PROBE_RESOURCE_PACK'):
        env.pop(key, None)
    for key in ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'TMP', 'TEMP'):
        folder = run / key.lower()
        folder.mkdir()
        env[key] = str(folder)
    observations, failures, screenshots = [], [], []

    def ctl(*words, timeout=60):
        words = list(map(str, words))
        reply = subprocess.run([str(controller), '--socket', str(endpoint), '--timeout', str(timeout), *words],
                               capture_output=True, text=True, timeout=timeout + 5)
        with (run / 'ctl.jsonl').open('a', encoding='utf-8', newline='\n') as output:
            output.write(json.dumps({'args': words, 'code': reply.returncode, 'stdout': reply.stdout, 'stderr': reply.stderr}) + '\n')
        if reply.returncode:
            raise RuntimeError(f'control failed {words}: {reply.stdout} {reply.stderr}; run={run}')
        result = json.loads(reply.stdout)
        if result.get('error') or result.get('ok') is False:
            raise RuntimeError(f'control rejected {words}: {result}; run={run}')
        return result

    def block(pos):
        return ctl('getblock', *pos)['block']

    def check(condition, message):
        if not condition:
            failures.append(message)

    def photo(name, target, camera):
        # Keep the camera fixed while the screenshot settles; a creative player
        # teleported into air otherwise falls away from the floating fixture.
        ctl('command', 'gamemode spectator')
        ctl('command', f'tp @s {camera[0]} {camera[1]} {camera[2]}')
        ctl('look-at', *target)
        ctl('wait', 30)
        ctl('key', 'f1', 'tap')
        try:
            path = run / (name + '.png')
            ctl('screenshot', path)
            if not path.is_file() or not path.stat().st_size:
                raise RuntimeError(f'missing screenshot {path}')
            screenshots.append(str(path))
        finally:
            ctl('key', 'f1', 'tap')
            ctl('command', 'gamemode creative')

    log = run / 'game.log'
    with log.open('w', encoding='utf-8', newline='\n') as output:
        proc = subprocess.Popen([str(game), '--resources', str(resources), '--dev-mod', str(content), '--control-socket', str(endpoint),
            '--offscreen', '--world', 'SoilTrees', '--seed', '42', '--creative', '--test-terrain',
            '--render-distance', '3', '--width', '1280', '--height', '720'], cwd=run, env=env,
            stdout=output, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic() + 120
            while not endpoint.exists():
                if proc.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError(f'game did not start; run={run}')
                time.sleep(.2)
            ctl('wait-ready', 120, timeout=125)
            for command in ('clear @s', 'time set 6000', 'weather clear', 'gamerule random_tick_speed 0',
                            'give @s minecraft:poppy 64', 'give @s tconstruct:earth_slime_fern 64',
                            'give @s minecraft:cactus 64', 'give @s minecraft:bone_meal 64'):
                ctl('command', command)
            for index, soil in enumerate(('earth', 'sky', 'ichor', 'ender')):
                x = index * 8
                ground = f'tconstruct:{soil}_slime_dirt'
                ctl('command', f'tp @s {x + .5} 110 3.5')
                ctl('wait', 40)
                ctl('command', f'fill {x-3} 99 -3 {x+3} 99 4 minecraft:stone')
                ctl('command', f'fill {x-3} 100 -3 {x+3} 105 4 minecraft:air')
                ctl('setblock', x, 100, 0, ground)
                ctl('setblock', x, 100, 3, 'minecraft:stone')
                ctl('command', f'tp @s {x+.5} 101 3.5')
                for plant, allowed in (('minecraft:poppy', True), ('tconstruct:earth_slime_fern', True), ('minecraft:cactus', False)):
                    ctl('command', f'tp @s {x+.5} 101 3.5')
                    ctl('wait', 8)
                    ctl('setblock', x, 101, 0, 'minecraft:air')
                    ctl('select', plant)
                    ctl('look-at-block', x, 100, 0)
                    ctl('interact', x, 100, 0, 'up')
                    ctl('wait', 20)
                    actual = block([x, 101, 0])
                    check((actual.split('[', 1)[0] == plant) if allowed else actual == 'minecraft:air',
                          f'{ground} / {plant}: expected allowed={allowed}, got {actual}')
                    observations.append({'soil': ground, 'plant': plant, 'allowed': allowed, 'actual': actual})
                    if plant == 'tconstruct:earth_slime_fern':
                        photo('soil-' + soil, [x+.5, 100.5, .5], [x+.5, 103, 4.5])

            for x, kind, log_id in ((128, 'earth', 'greenheart_log'), (176, 'sky', 'skyroot_log')):
                ctl('command', f'tp @s {x+.5} 140 15.5')
                ctl('wait', 80)
                ctl('command', f'fill {x-8} 99 -8 {x+8} 99 8 minecraft:stone')
                ctl('command', f'fill {x-8} 100 -8 {x+8} 135 8 minecraft:air')
                ctl('setblock', x, 100, 0, 'tconstruct:earth_slime_dirt')
                sapling = f'tconstruct:{kind}_slime_sapling'
                ctl('setblock', x, 101, 0, sapling + '[stage=1]')
                ctl('setblock', x, 100, 3, 'minecraft:stone')
                ctl('command', f'tp @s {x+.5} 101 3.5')
                ctl('select', 'minecraft:bone_meal')
                for clicks in range(1, 41):
                    ctl('look-at-block', x, 101, 0)
                    ctl('interact', x, 101, 0, 'up')
                    ctl('wait', 8)
                    if block([x, 101, 0]).split('[', 1)[0] != sapling:
                        break
                trunk = 'tconstruct:' + log_id
                height = 0
                while height < 24 and block([x, 101 + height, 0]).split('[', 1)[0] == trunk:
                    height += 1
                check(height in (range(4, 7) if kind == 'earth' else (5, 6, 7, 8, 10, 12, 14, 16)), f'{kind}: invalid trunk height {height}')
                if height:
                    logs, leaves, holes = canopy([x, 101, 0], height)
                    leaf_id = f'tconstruct:{kind}_slime_leaves'
                    check(block([x, 100, 0]) == 'tconstruct:earth_slime_dirt', f'{kind}: slime soil converted to vanilla dirt')
                    check(ctl('find-block', trunk, 32).get('count') == len(logs), f'{kind}: extra or missing logs')
                    check(ctl('find-block', leaf_id, 32).get('count') == len(leaves), f'{kind}: extra or missing leaves')
                    distances = Counter()
                    for pos in sorted(logs):
                        check(block(pos).split('[', 1)[0] == trunk, f'{kind}: missing log at {pos}')
                    for pos in sorted(leaves):
                        actual = block(pos)
                        check(actual.split('[', 1)[0] == leaf_id, f'{kind}: missing canopy at {pos}: {actual}')
                        properties = dict(item.split('=', 1) for item in actual.partition('[')[2].rstrip(']').split(',') if '=' in item)
                        distances[properties.get('distance', 'absent')] += 1
                    for pos in holes:
                        check(block(pos) == 'minecraft:air', f'{kind}: missing canopy cut at {pos}')
                    check('absent' not in distances and set(distances) <= set('1234567'), f'{kind}: invalid leaf distances {distances}')
                    observations.append({'tree': kind, 'height': height, 'clicks': clicks, 'expected_logs': len(logs),
                                         'expected_leaves': len(leaves), 'leaf_distances': dict(distances)})
                photo('tree-' + kind, [x+.5, 101+height/2, .5], [x+10.5, 105+height, 12.5])
            ctl('quit')
            proc.wait(timeout=60)
            check(proc.returncode == 0, f'game exit {proc.returncode}')
        finally:
            if proc.poll() is None:
                proc.terminate()
                try:
                    proc.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait(timeout=15)
    text = log.read_text(encoding='utf-8', errors='replace')
    for status in ('starting', 'stopping', 'stopped'):
        check(text.count(f'PLUTO-PROBE server_{status} dedicated=false') == 1, f'unclean lifecycle {status}')
    result = {'result': 'SOIL TREES FAIL' if failures else 'SOIL TREES PASS', 'run': str(run),
              'observations': observations, 'failures': failures, 'screenshots': screenshots,
              'scope': 'production soil item placement and sapling canopy; exact leaf-distance/RNG/flags, natural tree routing, save/LAN not proved'}
    (run / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8', newline='\n')
    print(json.dumps(result, indent=2))
    if failures:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
