#!/usr/bin/env python3
"""Real bone-meal growth through BlockEditor.place_feature; not custom-worldgen parity.

Uses isolated homes/worlds and the actual native build report. Four cases cover
both slime fungi on two different native configured-feature bases. Inspect the
saved images separately; this gate asserts states, not rendered pixels.
"""
import argparse
import json
import os
from pathlib import Path
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
    args = parser.parse_args()
    game, controller, resources = args.game.resolve(), args.ctl.resolve(), args.resources.resolve()
    if not game.is_file() or not controller.is_file() or not resources.is_dir():
        parser.error('--game/--ctl must be files; --resources must be a prepared directory')
    pins = json.loads((ROOT / 'tconstruct/compatibility.json').read_text(encoding='utf-8'))
    require_report(game.parent, pins['horizon_revision'], pins['mixins_revision'], {'pluto_lib', 'pluto_probe', 'tconstruct'})
    runs = ROOT / '.local/smoke-runs'
    runs.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix='feature-growth-', dir=runs)).resolve()
    endpoint, game_log = run / 'control.json', run / 'game.log'
    env = dict(os.environ, HORIZON_NO_AUDIO='1', HORIZON_NO_DISCORD='1',
               HORIZON_TELEMETRY_URL='off', HORIZON_LAUNCHER_NO_SELF_UPDATE='1', RUST_MIN_STACK='16777216')
    for key in ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME'):
        folder = run / key.lower()
        folder.mkdir()
        env[key] = str(folder)
    temporary = run / 'tmp'
    temporary.mkdir()
    env['TMP'] = env['TEMP'] = str(temporary)
    env.pop('PLUTO_PROBE_RESOURCE_PACK', None)
    env.pop('PLUTO_PROBE_FEATURE_MODE', None)
    observations, screenshots = [], []

    def ctl(*words, timeout=40):
        words = tuple(map(str, words))
        reply = subprocess.run([str(controller), '--socket', str(endpoint), '--timeout', str(timeout), *words],
                               capture_output=True, text=True, timeout=timeout + 5)
        with (run / 'ctl.jsonl').open('a', encoding='utf-8', newline='\n') as output:
            output.write(json.dumps({'args': words, 'code': reply.returncode, 'stdout': reply.stdout, 'stderr': reply.stderr}) + '\n')
        if reply.returncode:
            raise RuntimeError(f'control failed: {words}: {reply.stdout} {reply.stderr}; run={run}')
        value = json.loads(reply.stdout)
        if not isinstance(value, dict) or value.get('error') or value.get('ok') is False:
            raise RuntimeError(f'control rejected {words}: {value}; run={run}')
        return value

    def block(x, y, z):
        return ctl('getblock', x, y, z)['block'].split('[', 1)[0]

    with game_log.open('w', encoding='utf-8', newline='\n') as output:
        proc = subprocess.Popen([str(game), '--resources', str(resources), '--control-socket', str(endpoint),
                                 '--offscreen', '--world', 'FeatureGrowthProbe', '--seed', '42', '--creative',
                                 '--test-terrain', '--render-distance', '2', '--width', '1280', '--height', '720'],
                                cwd=run, env=env, stdout=output, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic() + 120
            while not endpoint.exists():
                if proc.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError(f'game did not start: {game_log}')
                time.sleep(.2)
            ctl('wait-ready', 120, timeout=125)
            for command in ('clear @s', 'time set 6000', 'weather clear', 'gamerule random_tick_speed 0',
                            'give @s minecraft:bone_meal 64'):
                ctl('command', command)
            for foliage, soil in (('blood', 'earth'), ('blood', 'ichor'), ('ichor', 'earth'), ('ichor', 'ichor')):
                ground = f'tconstruct:{soil}_slime_dirt'
                sapling = f'tconstruct:{foliage}_slime_sapling'
                for command in ('fill -9 99 -9 9 99 9 minecraft:stone',
                                'fill -9 100 -9 9 132 9 minecraft:air',
                                f'setblock 0 100 0 {ground}', f'setblock 0 101 0 {sapling}',
                                'setblock 0 101 3 minecraft:stone', 'tp @s 0.5 102 3.5'):
                    ctl('command', command)
                ctl('wait', 20)
                if block(0, 100, 0) != ground or block(0, 101, 0) != sapling:
                    raise RuntimeError(f'fixture did not survive: {foliage}/{soil}; run={run}')
                ctl('select', 'minecraft:bone_meal')
                clicks = 0
                deadline = time.monotonic() + 90
                while block(0, 101, 0) == sapling and clicks < 32 and time.monotonic() < deadline:
                    ctl('look-at', 'block', 0, 101, 0)
                    ctl('interact', 0, 101, 0, 'up')
                    ctl('wait', 4)
                    clicks += 1
                column = [block(0, y, 0) for y in range(101, 132)]
                expected_log = 'tconstruct:bloodshroom_log'
                expected_cap = f'tconstruct:{foliage}_slime_leaves'
                if column.count(expected_log) < 2:
                    raise RuntimeError(f'no fungus stem after {clicks} clicks: {foliage}/{soil}: {column}; run={run}')
                top = 101 + max(index for index, name in enumerate(column) if name == expected_log)
                # Native hat placement is random: a single center pixel need not
                # be a leaf. Observe a small upper-cap region, not one lucky cell.
                cap = next(((x, y, z) for y in range(top + 1, max(100, top - 3), -1)
                            for x in range(-1, 2) for z in range(-1, 2)
                            if block(x, y, z) == expected_cap), None)
                if cap is None:
                    raise RuntimeError(f'no matching upper cap: {foliage}/{soil}; run={run}')
                if block(0, 100, 0) != ground:
                    raise RuntimeError(f'fungus changed its configured base: {foliage}/{soil}; run={run}')
                height = top - 100
                camera_y = 100 + height // 2
                camera_z = max(12, height * 2)
                ctl('setblock', 0, camera_y - 1, camera_z, 'minecraft:stone')
                ctl('command', f'tp @s 0.5 {camera_y} {camera_z + 0.5}')
                ctl('command', 'effect clear @s')
                ctl('look', 180, 0)
                ctl('wait', 40)
                ctl('key', 'f1', 'tap')
                try:
                    image = run / f'{foliage}-{soil}-fungus.png'
                    ctl('screenshot', image)
                    if not image.is_file() or not image.stat().st_size:
                        raise RuntimeError(f'screenshot missing: {image}')
                    screenshots.append(str(image))
                finally:
                    ctl('key', 'f1', 'tap')
                observations.append({'foliage': foliage, 'base': ground, 'clicks': clicks,
                                     'center_log_count': column.count(expected_log), 'center_cap': expected_cap})
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
    lines = game_log.read_text(encoding='utf-8', errors='replace').splitlines()
    lifecycle = [f'PLUTO-PROBE server_{phase} dedicated=false' for phase in ('starting', 'stopping', 'stopped')]
    if any(lines.count(event) != 1 for event in lifecycle) or [lines.index(event) for event in lifecycle] != sorted(lines.index(event) for event in lifecycle):
        raise RuntimeError(f'lifecycle must occur once and in order: {game_log}')
    print(json.dumps({'result': 'LIVE FEATURE GROWTH PASS', 'run': str(run), 'observations': observations,
                      'screenshots': screenshots, 'image_review_required': True}, indent=2))


if __name__ == '__main__':
    main()
