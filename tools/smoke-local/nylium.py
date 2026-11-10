#!/usr/bin/env python3
"""Focused real-game nylium gate; requires manual review of saved screenshots.

No tests/builds are launched here. Bone meal uses real select/look-at/interact;
commands arrange fixtures only. Random outcomes are retried within deadlines,
not required to match a particular seed or yield a rare sapling on one click.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
HORIZON = '1453e0e614af4b2d04dd0d228a94162b5d459910'
MIXINS = 'e59da9094f0c2fdda6c627fe9eff22dec6a906f0'
SOILS = ('earth', 'sky', 'ichor', 'ender', 'vanilla')
VARIANTS = [(foliage, soil) for foliage in ('ichor', 'blood') for soil in SOILS]


def grass(foliage, soil):
    return f'tconstruct:{foliage}_{soil}_slime_grass'


def dirt(soil):
    return 'minecraft:dirt' if soil == 'vanilla' else f'tconstruct:{soil}_slime_dirt'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--game', type=Path, required=True)
    parser.add_argument('--ctl', type=Path, required=True)
    parser.add_argument('--resources', type=Path, required=True)
    args = parser.parse_args()
    game, controller, resources = args.game.resolve(), args.ctl.resolve(), args.resources.resolve()
    if not game.is_file() or not controller.is_file() or not resources.is_dir():
        parser.error('--game/--ctl must be files; --resources must be a prepared directory')
    report = json.loads((game.parent / 'native-mods.json').read_text(encoding='utf-8'))
    built = {mod['id'] for mod in report.get('mods', [])}
    if report.get('horizon') != HORIZON or report.get('horizon_mixins') != MIXINS:
        raise RuntimeError('nylium gate requires the reviewed Horizon/Mixins pins')
    if report.get('left_out') != [] or not {'pluto_lib', 'pluto_probe', 'tconstruct'} <= built:
        raise RuntimeError('required native mods must be present with left_out=[]')
    runs = ROOT / '.local/smoke-runs'
    runs.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix='nylium-', dir=runs)).resolve()
    endpoint, game_log = run / 'control.json', run / 'game.log'
    env = dict(os.environ, HORIZON_NO_AUDIO='1', HORIZON_NO_DISCORD='1',
               HORIZON_TELEMETRY_URL='off', HORIZON_LAUNCHER_NO_SELF_UPDATE='1',
               RUST_MIN_STACK='16777216')
    for key in ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME'):
        folder = run / key.lower()
        folder.mkdir()
        env[key] = str(folder)
    temporary = run / 'tmp'
    temporary.mkdir()
    env['TMP'] = env['TEMP'] = str(temporary)
    env.pop('PLUTO_PROBE_RESOURCE_PACK', None)
    observations, screenshots = [], []
    phase_deadline = None

    def record(data):
        with (run / 'ctl.jsonl').open('a', encoding='utf-8', newline='\n') as output:
            output.write(json.dumps(data, ensure_ascii=False) + '\n')

    def ctl(*words, timeout=40):
        words = tuple(map(str, words))
        if phase_deadline is not None:
            remaining = phase_deadline - time.monotonic()
            if remaining <= 0:
                raise RuntimeError(f'phase deadline exceeded before {words}; run={run}')
            timeout = min(timeout, max(1, int(remaining)))
        result = subprocess.run([str(controller), '--socket', str(endpoint), '--timeout',
                                 str(timeout), *map(str, words)], capture_output=True,
                                text=True, timeout=timeout + 5)
        record({'args': words, 'code': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr})
        if result.returncode:
            raise RuntimeError(f'control failed: {words}: {result.stdout} {result.stderr}; run={run}')
        reply = json.loads(result.stdout)
        if not isinstance(reply, dict) or reply.get('error') or reply.get('ok') is False:
            raise RuntimeError(f'control rejected {words}: {reply}; run={run}')
        return reply

    def command(text):
        return ctl('command', text)

    def block(x, y, z):
        return ctl('getblock', x, y, z)['block'].split('[', 1)[0]

    def require(actual, expected, label):
        if actual != expected:
            raise RuntimeError(f'{label}: expected {expected!r}, got {actual!r}; run={run}')

    def screenshot(name):
        # Photograph the whole bed from outside it, not from inside a new plant.
        # This camera support is outside the asserted 5x5 bed and the next fixture clears it.
        if name.endswith('-bonemeal.png'):
            ctl('setblock', 0, 101, -3, 'minecraft:stone')
            command('tp @s 0.5 102 -2.5')
            ctl('look', 0, 45)
        ctl('wait', 20)
        ctl('key', 'f1', 'tap')
        try:
            path = run / name
            ctl('screenshot', path)
            if not path.is_file() or not path.stat().st_size:
                raise RuntimeError(f'screenshot missing: {path}')
            screenshots.append(str(path))
        finally:
            ctl('key', 'f1', 'tap')

    def tick_speed():
        # Integrated owner_console_command returns newly captured plain chat lines.
        reply = command('gamerule minecraft:random_tick_speed')
        chat = reply.get('chat', [])
        if not chat or not isinstance(chat[-1], str):
            raise RuntimeError(f'cannot read original random_tick_speed: {reply}')
        values = re.findall(r'(?<![\w])\d+(?![\w])', chat[-1])
        if len(values) != 1:
            raise RuntimeError(f'ambiguous random_tick_speed query: {reply}')
        return int(values[0])

    def set_speed(value):
        command(f'gamerule minecraft:random_tick_speed {value}')
        require(tick_speed(), value, 'random_tick_speed write')

    def click(x, y, z, face='up'):
        ctl('select', 'minecraft:bone_meal')
        ctl('look-at', 'block', x, y, z)
        ctl('interact', x, y, z, face)
        ctl('wait', '4')

    def snapshot():
        return {(x, y, z): block(x, y, z)
                for y in (100, 101) for x in range(-2, 3) for z in range(-2, 3)}

    original_speed = None
    with game_log.open('w', encoding='utf-8', newline='\n') as output:
        proc = subprocess.Popen([str(game), '--resources', str(resources), '--control-socket',
                                 str(endpoint), '--offscreen', '--world', 'NyliumProbe', '--seed',
                                 '42', '--creative', '--test-terrain', '--render-distance', '2',
                                 '--width', '1280', '--height', '720'], cwd=run, env=env,
                                stdout=output, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic() + 120
            while not endpoint.exists():
                if proc.poll() is not None or time.monotonic() >= deadline:
                    raise RuntimeError(f'game startup failed: {game_log}')
                time.sleep(.2)
            ctl('wait-ready', '120', timeout=125)
            original_speed = tick_speed()
            set_speed(0)
            for text in ('clear @s', 'time set 6000', 'weather clear',
                         'fill -6 99 -6 32 99 8 minecraft:stone',
                         'fill -6 100 -6 32 108 8 minecraft:air', 'tp @s 12.5 101 -2.5'):
                command(text)
            # Paired covered/uncovered cases plus a transparent full cube and a
            # partial slab observation. All ten variants stay inside loaded chunks.
            for index, (foliage, soil) in enumerate(VARIANTS):
                x = index * 3
                for z in (0, 2, 4, 6):
                    ctl('setblock', x, 100, z, grass(foliage, soil))
                ctl('setblock', x, 101, 0, 'minecraft:stone')
                ctl('setblock', x, 101, 4, 'minecraft:glass')
                ctl('setblock', x, 101, 6, 'minecraft:stone_slab[type=bottom]')
            try:
                set_speed(512)
                deadline = time.monotonic() + 90
                phase_deadline = deadline
                while True:
                    pending = [(i, f, s) for i, (f, s) in enumerate(VARIANTS)
                               if block(i * 3, 100, 0) != dirt(s)]
                    if not pending:
                        break
                    if proc.poll() is not None or time.monotonic() >= deadline:
                        raise RuntimeError(f'covered nylium did not decay before 90s deadline: {pending}')
                    time.sleep(.1)
                # Do not treat a no-tick instant as evidence of uncovered survival.
                ctl('wait', '200')
                for index, (foliage, soil) in enumerate(VARIANTS):
                    x = index * 3
                    require(block(x, 100, 0), dirt(soil), f'{foliage}/{soil} shade decay')
                    for z in (2, 4):
                        require(block(x, 100, z), grass(foliage, soil), f'{foliage}/{soil} air/glass survival')
                    observations.append({'variant': grass(foliage, soil), 'shade': 'own dirt',
                                         'air_and_glass': 'unchanged',
                                         'bottom_slab_observed': block(x, 100, 6)})
                command('tp @s 12.5 100 3.5')
                ctl('look-at', 'block', '12', '100', '2')
                screenshot('shade-and-boundaries.png')
            finally:
                phase_deadline = None
                if proc.poll() is None:
                    set_speed(original_speed)
            # No random ticks may interfere with the bone-meal-only postconditions.
            set_speed(0)
            command('give @s minecraft:bone_meal 64')
            for foliage, soil in VARIANTS:
                command('fill -4 100 -4 4 108 4 minecraft:air')
                ctl('fill', '-2', '100', '-2', '2', '100', '2', dirt(soil))
                ctl('setblock', '0', '100', '0', grass(foliage, soil))
                command('tp @s 0.5 100 -2.5')
                # Glass occupies the required air above without hiding the north
                # face. An actual right click must leave the entire bed unchanged.
                ctl('setblock', '0', '101', '0', 'minecraft:glass')
                # Open a sight corridor to the north face; otherwise the adjacent
                # same-height dirt would intercept the actual right-click ray.
                for z in (-2, -1):
                    ctl('setblock', '0', '100', z, 'minecraft:air')
                before = snapshot()
                click(0, 100, 0, 'north')
                require(snapshot(), before, f'{foliage}/{soil} occupied-above rejection')
                for z in (-2, -1):
                    ctl('setblock', '0', '100', z, dirt(soil))
                ctl('setblock', '0', '101', '0', 'minecraft:air')
                # Sentinel is outside the dirt bed. Nylium bone meal must not
                # replace this already-grassed, different-foliage support block.
                ctl('setblock', '3', '100', '0', grass('earth', soil))
                expected_plants = {f'tconstruct:{foliage}_{kind}'
                                   for kind in ('slime_fern', 'slime_tall_grass', 'slime_sapling')}
                plants, spread = {}, {}
                deadline = time.monotonic() + 120
                phase_deadline = deadline
                for attempt in range(1, 17):
                    if time.monotonic() >= deadline:
                        raise RuntimeError(f'bone meal deadline exceeded before click: {foliage}/{soil}')
                    # A previous click may have planted the center. Remove only
                    # that obstruction so the next real click still targets nylium.
                    ctl('setblock', '0', '101', '0', 'minecraft:air')
                    click(0, 100, 0)
                    states = snapshot()
                    plants = {str(pos): state for pos, state in states.items()
                              if pos[1] == 101 and state != 'minecraft:air'}
                    unexpected = set(plants.values()) - expected_plants
                    if unexpected:
                        raise RuntimeError(f'wrong-foliage/unknown plants: {unexpected}; run={run}')
                    spread = {str(pos): state for pos, state in states.items()
                              if pos[1] == 100 and pos != (0, 100, 0)
                              and state == grass(foliage, soil)}
                    allowed_base = {dirt(soil), grass(foliage, soil)}
                    if any(state not in allowed_base for pos, state in states.items() if pos[1] == 100):
                        raise RuntimeError(f'wrong-foliage soil conversion: {states}; run={run}')
                    require(block(3, 100, 0), grass('earth', soil), 'other-foliage sentinel')
                    if plants and spread:
                        break
                    if time.monotonic() >= deadline:
                        raise RuntimeError(f'bone meal deadline exceeded: {foliage}/{soil}; plants={plants}, spread={spread}')
                else:
                    raise RuntimeError(f'16 actual clicks did not produce plants AND spread: {foliage}/{soil}')
                phase_deadline = None
                observations.append({'variant': grass(foliage, soil), 'occupied_rejected': True,
                                     'clicks': attempt, 'plants': plants, 'spread': spread,
                                     'other_foliage_sentinel': 'unchanged'})
                screenshot(f'{foliage}-{soil}-bonemeal.png')
            set_speed(original_speed)
            ctl('quit')
            proc.wait(timeout=60)
            require(proc.returncode, 0, 'normal game exit')
        finally:
            phase_deadline = None
            if proc.poll() is None:
                try:
                    if original_speed is not None:
                        set_speed(original_speed)
                    ctl('quit', timeout=15)
                    proc.wait(timeout=30)
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
    if any(lines.count(event) != 1 for event in lifecycle):
        raise RuntimeError(f'lifecycle events must occur once: {game_log}')
    if [lines.index(event) for event in lifecycle] != sorted(lines.index(event) for event in lifecycle):
        raise RuntimeError(f'lifecycle events out of order: {game_log}')
    result = {'result': 'NYLIUM STATE GATE PASS', 'run': str(run), 'observations': observations,
              'screenshots': screenshots, 'original_random_tick_speed': original_speed,
              'image_review_required': True,
              'unverified': ['bottom-slab Java parity (observations only)',
                             'exact RNG probabilities, every rare plant outcome, LAN, save/reload']}
    (run / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8', newline='\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
