#!/usr/bin/env python3
"""Focused world-mesh color probe; screenshots still require a human/agent image review."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

from client_resources import rgba_png
from native_build_report import require_report

ROOT = Path(__file__).resolve().parents[2]
HORIZON = 'ebcfa2263aa3d8e399aa993b1e05daf42eb4cfa0'
MIXINS = 'e59da9094f0c2fdda6c627fe9eff22dec6a906f0'
COLOR_EVENT = 'PLUTO-PROBE block_color pos=[0, 101, 0] tint=0 persistent=Some("true")'
FALLBACK_EVENT = 'PLUTO-PROBE block_color fallback pos=[3, 101, 0]'
FOLIAGE = ('earth', 'sky', 'ichor', 'ender', 'blood')
PALETTE = ((224, 64, 64), (64, 224, 64), (64, 64, 224), (224, 224, 64), (64, 224, 224))


def write_colormaps(pack, palette, malformed_ichor=False):
    folder = pack / 'assets/tconstruct/textures/colormap'
    folder.mkdir(parents=True, exist_ok=True)
    for name, rgb in zip(FOLIAGE, palette):
        invalid = malformed_ichor and name == 'ichor'
        width = 255 if invalid else 256
        rgb = (240, 0, 240) if invalid else rgb
        pixels = bytes((*rgb, 255)) * (width * 256)
        (folder / f'{name}_grass_color.png').write_bytes(rgba_png(pixels, width, 256))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--game', type=Path, required=True)
    parser.add_argument('--ctl', type=Path, required=True)
    parser.add_argument('--resources', type=Path, required=True)
    parser.add_argument('--tinker-pack', action='store_true', help='override the 47-block scene with five controlled maps, F3+T, and malformed-map retention')
    args = parser.parse_args()
    game, controller, resources = args.game.resolve(), args.ctl.resolve(), args.resources.resolve()
    if not game.is_file() or not controller.is_file() or not resources.is_dir():
        parser.error('--game/--ctl must be files and --resources must be a prepared resource directory')
    require_report(game.parent, HORIZON, MIXINS, {'pluto_lib', 'pluto_probe', 'tconstruct'})

    runs = ROOT / '.local/smoke-runs'
    runs.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix='block-colors-', dir=runs)).resolve()
    endpoint, game_log = run / 'control.json', run / 'game.log'
    env = dict(os.environ, HORIZON_NO_AUDIO='1', HORIZON_NO_DISCORD='1', HORIZON_TELEMETRY_URL='off', HORIZON_LAUNCHER_NO_SELF_UPDATE='1', RUST_MIN_STACK='16777216')
    for key in ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME'):
        folder = run / key.lower()
        folder.mkdir()
        env[key] = str(folder)
    temporary = run / 'tmp'
    temporary.mkdir()
    env['TMP'] = env['TEMP'] = str(temporary)
    pack = run / 'resourcepacks/PlutoColorMaps'
    if args.tinker_pack:
        pack.mkdir(parents=True)
        (pack / 'pack.mcmeta').write_bytes(json.dumps({'pack': {'pack_format': 97, 'min_format': [97, 1], 'max_format': [97, 1], 'description': 'Isolated color-map fixture'}}).encode('utf-8'))
        write_colormaps(pack, PALETTE)
        (run / 'options.txt').write_bytes(b'resourcePacks:["vanilla","file/PlutoColorMaps"]\n')

    def ctl(*words, timeout=40):
        result = subprocess.run([str(controller), '--socket', str(endpoint), '--timeout', str(timeout), *map(str, words)], capture_output=True, text=True, timeout=timeout + 5)
        with (run / 'ctl.jsonl').open('a', encoding='utf-8', newline='\n') as log:
            log.write(json.dumps({'args': words, 'code': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr}, ensure_ascii=False) + '\n')
        if result.returncode:
            raise RuntimeError(f'control operation failed: {words}: {result.stdout} {result.stderr}; run={run}')
        reply = json.loads(result.stdout)
        if not isinstance(reply, dict) or reply.get('error') or reply.get('ok') is False:
            raise RuntimeError(f'control operation rejected: {words}: {reply}; run={run}')
        return reply

    screenshots = []

    def take_screenshot(name):
        path = run / name
        ctl('key', 'f1', 'tap')
        try:
            ctl('wait', '5')
            ctl('screenshot', str(path))
            if not path.is_file() or not path.stat().st_size:
                raise RuntimeError(f'the screenshot was not saved: {path}')
            screenshots.append(str(path))
        finally:
            ctl('key', 'f1', 'tap')

    def photograph_tinker(stage):
        take_screenshot(f'tinker-colors-{stage}.png')
        # A separate, temporary camera support makes the smaller plant models legible.
        ctl('setblock', 6, 105, 9, 'minecraft:stone')
        try:
            ctl('command', 'tp @s 6.5 106 9.5')
            ctl('look', 0, 25)
            ctl('wait', '40')
            take_screenshot(f'tinker-plants-{stage}.png')
        finally:
            ctl('command', 'tp @s 6.5 112 -8.5')
            ctl('look', 0, 35)
            ctl('setblock', 6, 105, 9, 'minecraft:air')
            ctl('wait', '40')

    def verify_placements(placed):
        for position, expected, _ in placed:
            actual = ctl('getblock', *position)['block'].split('[', 1)[0]
            if actual != expected:
                raise RuntimeError(f'color fixture {position}: expected {expected}, got {actual}; run={run}')

    def tinker_fixture():
        ctl('command', 'fill -4 99 -4 18 99 32 minecraft:stone')
        ctl('command', 'fill -4 100 -4 18 110 32 minecraft:air')
        placed = []

        def place(x, y, z, state, count=True):
            ctl('setblock', x, y, z, state)
            placed.append(((x, y, z), state.split('[', 1)[0], count))

        for column, foliage in enumerate(FOLIAGE):
            x = column * 3
            for row, soil in enumerate(('vanilla', 'earth', 'sky', 'ender', 'ichor')):
                place(x, 100, row * 3, f'tconstruct:{foliage}_{soil}_slime_grass')
            for z in (15, 18):
                place(x, 100, z, f'tconstruct:{foliage}_vanilla_slime_grass', False)
            place(x, 101, 15, f'tconstruct:{foliage}_slime_fern')
            # Original SlimeTallGrassBlock extends BushBlock, not a two-block plant.
            place(x, 101, 18, f'tconstruct:{foliage}_slime_tall_grass')
            place(x, 100, 21, 'minecraft:stone', False)
            place(x, 101, 21, f'tconstruct:potted_{foliage}_slime_fern')
            # Ichor/blood foliage has no persistent property; random ticks are disabled.
            place(x, 101, 24, f'tconstruct:{foliage}_slime_leaves')
        for x, foliage in ((3, 'sky'), (9, 'ender')):
            place(x, 101, 28, 'minecraft:stone', False)
            place(x, 101, 27, f'tconstruct:{foliage}_slime_vine[south=true]')
        ctl('setblock', 6, 111, -9, 'minecraft:stone')
        ctl('command', 'tp @s 6.5 112 -8.5')
        ctl('look', 0, 35)
        ctl('wait', '60')
        verify_placements(placed)
        names = sorted({name for _, name, count in placed if count})
        if len(names) != 47:
            raise RuntimeError(f'expected the 47 original Java block registrations, got {len(names)}')
        return names, placed

    def reload_client(count):
        ctl('key', 'f3', 'down')
        try:
            ctl('key', 't', 'tap', timeout=125)
        finally:
            if proc.poll() is None:
                ctl('key', 'f3', 'up', timeout=125)
        marker = f'PLUTO-PROBE client_resources count={count} pass=true'
        deadline = time.monotonic() + 120
        while marker not in game_log.read_text(encoding='utf-8', errors='replace').splitlines():
            if proc.poll() is not None or time.monotonic() > deadline:
                raise RuntimeError(f'resource reload {count} did not complete; log={game_log}')
            time.sleep(.2)
        ctl('wait-ready', '120', timeout=125)
        ctl('close')
        if ctl('status')['debug']['overlay']:
            ctl('key', 'f3', 'tap')
        ctl('wait', '60')

    tinker_blocks = []
    with game_log.open('w', encoding='utf-8', newline='\n') as output:
        proc = subprocess.Popen([str(game), '--resources', str(resources), '--control-socket', str(endpoint), '--offscreen', '--world', 'BlockColorsProbe', '--seed', '42', '--creative', '--test-terrain', '--render-distance', '2', '--width', '1280', '--height', '720'], cwd=run, env=env, stdout=output, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic() + 120
            while not endpoint.exists():
                if proc.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError(f'game did not start: {game_log}')
                time.sleep(.2)
            ctl('wait-ready', '120', timeout=125)
            for command in (
                'clear @s', 'time set 6000', 'weather clear', 'gamerule random_tick_speed 0',
                'fill -4 99 -4 10 99 10 minecraft:stone',
                'fill -4 100 -4 10 110 10 minecraft:air',
                'setblock 0 101 0 minecraft:oak_leaves[persistent=true]',
                'setblock 3 101 0 minecraft:oak_leaves[persistent=true]',
                'setblock 6 101 0 minecraft:stone',
                'tp @s 3.5 101 3.5',
            ):
                ctl('command', command)
            ctl('wait', '10')
            ctl('look-at', 'block', '3', '101', '0')
            deadline = time.monotonic() + 60
            while True:
                observed = game_log.read_text(encoding='utf-8', errors='replace').splitlines()
                missing = [event for event in (COLOR_EVENT, FALLBACK_EVENT) if event not in observed]
                if not missing:
                    break
                if proc.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError(f'color callbacks were not observed: {missing}; log={game_log}')
                time.sleep(.1)
            # Allow the completed section mesh to reach the rendered frame.
            ctl('wait', '40')
            take_screenshot('colors.png')
            tinker_blocks, placed = tinker_fixture()
            photograph_tinker('initial')
            if args.tinker_pack:
                reloaded = PALETTE[1:] + PALETTE[:1]
                write_colormaps(pack, reloaded)
                reload_client(2)
                verify_placements(placed)
                photograph_tinker('reloaded')
                write_colormaps(pack, reloaded, malformed_ichor=True)
                reload_client(3)
                verify_placements(placed)
                photograph_tinker('retained')
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
    if any(event not in lines for event in (COLOR_EVENT, FALLBACK_EVENT)):
        raise RuntimeError(f'color or fallback callback missing from the final log: {game_log}')
    lifecycle = [f'PLUTO-PROBE server_{phase} dedicated=false' for phase in ('starting', 'stopping', 'stopped')]
    if any(lines.count(event) != 1 for event in lifecycle):
        raise RuntimeError(f'lifecycle events must occur exactly once: {game_log}')
    if [lines.index(event) for event in lifecycle] != sorted(lines.index(event) for event in lifecycle):
        raise RuntimeError(f'lifecycle events are out of order: {game_log}')
    if args.tinker_pack:
        reloads = [line for line in lines if line.startswith('PLUTO-PROBE client_resources count=')]
        expected = [f'PLUTO-PROBE client_resources count={count} pass=true' for count in (1, 2, 3)]
        if reloads != expected:
            raise RuntimeError(f'expected one initial and two completed client reloads, got {reloads}')
        failure = 'tconstruct: could not load 256x256 slime colormap tconstruct:textures/colormap/ichor_grass_color.png; keeping previous colors'
        if failure not in lines:
            raise RuntimeError(f'malformed Ichor map was not rejected by the consumer: {game_log}')
    print(json.dumps({'result': 'BLOCK COLORS PROBE PASS', 'run': str(run), 'screenshots': screenshots, 'callbacks': [COLOR_EVENT, FALLBACK_EVENT], 'tinker_blocks': tinker_blocks, 'world_x_columns': FOLIAGE, 'initial_palette': PALETTE if args.tinker_pack else [], 'reloaded_palette': PALETTE[1:] + PALETTE[:1] if args.tinker_pack else [], 'image_review_required': True}, indent=2))


if __name__ == '__main__':
    main()
