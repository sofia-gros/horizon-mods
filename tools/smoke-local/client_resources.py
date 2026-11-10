#!/usr/bin/env python3
"""Focused client resource baseline/F3+T gate, optionally exercising pack override and refresh."""
import argparse
import json
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import time
import zlib

ROOT = Path(__file__).resolve().parents[2]
HORIZON = '1453e0e614af4b2d04dd0d228a94162b5d459910'
MIXINS = 'e59da9094f0c2fdda6c627fe9eff22dec6a906f0'
EXPECTED = [f'PLUTO-PROBE client_resources count={count} pass=true' for count in (1, 2)]
PACK_EXPECTED = [f'PLUTO-PROBE client_pack count={count} pass=true' for count in (1, 2)]
INITIAL_PIXELS = bytes([200, 100, 50, 128, 0, 0, 255, 255])
RELOADED_PIXELS = bytes([1, 2, 3, 255, 4, 5, 6, 64])


def rgba_png(pixels, width=1, height=2):
    """Encode top-first, unpremultiplied RGBA; the API probe defaults to 1x2."""
    if width <= 0 or height <= 0 or len(pixels) != width * height * 4:
        raise ValueError('PNG dimensions must match the RGBA byte count')

    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))

    stride = width * 4
    rows = b''.join(b'\0' + pixels[start:start + stride] for start in range(0, len(pixels), stride))
    return (b'\x89PNG\r\n\x1a\n'
            + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(rows)) + chunk(b'IEND', b''))


def write_pack_assets(pack, raw, pixels):
    assets = pack / 'assets/pluto_probe'
    (assets / 'textures').mkdir(parents=True, exist_ok=True)
    (assets / 'probe.txt').write_bytes(raw)
    (assets / 'textures/resource_probe.png').write_bytes(rgba_png(pixels))


def write_stone_texture(pack, color):
    textures = pack / 'assets/minecraft/textures/block'
    textures.mkdir(parents=True, exist_ok=True)
    (textures / 'stone.png').write_bytes(rgba_png(bytes(color) * 256, 16, 16))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--game', type=Path, required=True)
    parser.add_argument('--ctl', type=Path, required=True)
    parser.add_argument('--resources', type=Path, required=True)
    parser.add_argument('--pack-override', action='store_true', help='select two local packs with conflicting resources, then edit the higher-priority pack before F3+T')
    args = parser.parse_args()
    game, controller, resources = args.game.resolve(), args.ctl.resolve(), args.resources.resolve()
    if not game.is_file() or not controller.is_file() or not resources.is_dir():
        parser.error('--game/--ctl must be files and --resources must be a prepared resource directory')
    report = json.loads((game.parent / 'native-mods.json').read_text(encoding='utf-8'))
    if report.get('horizon') != HORIZON or report.get('horizon_mixins') != MIXINS:
        raise RuntimeError('client resource gate requires the reviewed Horizon/Mixins pins')
    built = {mod['id'] for mod in report.get('mods', [])}
    if report.get('left_out') != [] or not {'pluto_lib', 'pluto_probe', 'tconstruct'} <= built:
        raise RuntimeError('required native mods must all be included with left_out=[]')

    runs = ROOT / '.local/smoke-runs'
    runs.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix='client-resources-', dir=runs)).resolve()
    endpoint, game_log = run / 'control.json', run / 'game.log'
    env = dict(os.environ, HORIZON_NO_AUDIO='1', HORIZON_NO_DISCORD='1', HORIZON_TELEMETRY_URL='off', HORIZON_LAUNCHER_NO_SELF_UPDATE='1', RUST_MIN_STACK='16777216')
    for key in ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME'):
        folder = run / key.lower()
        folder.mkdir()
        env[key] = str(folder)
    temporary = run / 'tmp'
    temporary.mkdir()
    env['TMP'] = env['TEMP'] = str(temporary)
    # The optional mode, not the caller's inherited environment, controls this probe.
    env.pop('PLUTO_PROBE_RESOURCE_PACK', None)
    if args.pack_override:
        env['PLUTO_PROBE_RESOURCE_PACK'] = '1'
        # Official horizon-core/version.rs pins client pack format (97, 1).
        # packs::validate_pack_range requires min/max for modern formats.
        metadata = {'pack': {'pack_format': 97, 'min_format': [97, 1], 'max_format': [97, 1], 'description': 'PlutoLib resource probe fixture'}}
        low_pack, high_pack = (run / 'resourcepacks' / name for name in ('PlutoProbeLow', 'PlutoProbe'))
        for pack in (low_pack, high_pack):
            pack.mkdir(parents=True)
            (pack / 'pack.mcmeta').write_text(json.dumps(metadata) + '\n', encoding='utf-8', newline='\n')
        # packs::enabled applies the options list bottom first; the later pack wins.
        write_pack_assets(low_pack, b'low', bytes(8))
        write_pack_assets(high_pack, b'initial', INITIAL_PIXELS)
        write_stone_texture(high_pack, [200, 100, 50, 255])
        selection = ['vanilla', 'file/PlutoProbeLow', 'file/PlutoProbe']
        (run / 'options.txt').write_text('version:5023\nresourcePacks:' + json.dumps(selection) + '\n', encoding='utf-8', newline='\n')

    def log_control(record):
        with (run / 'ctl.jsonl').open('a', encoding='utf-8', newline='\n') as log:
            log.write(json.dumps(record, ensure_ascii=False) + '\n')

    def ctl(*words, timeout=40):
        try:
            result = subprocess.run([str(controller), '--socket', str(endpoint), '--timeout', str(timeout), *map(str, words)], capture_output=True, text=True, timeout=timeout + 5)
        except subprocess.TimeoutExpired as error:
            def text(value):
                return value.decode('utf-8', errors='replace') if isinstance(value, bytes) else value
            log_control({'args': words, 'code': None, 'error': 'timeout', 'stdout': text(error.stdout), 'stderr': text(error.stderr)})
            raise RuntimeError(f'control operation timed out: {words}; run={run}') from error
        log_control({'args': words, 'code': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr})
        if result.returncode:
            raise RuntimeError(f'control operation failed: {words}: {result.stdout} {result.stderr}; run={run}')
        reply = json.loads(result.stdout)
        if not isinstance(reply, dict) or reply.get('error') or reply.get('ok') is False:
            raise RuntimeError(f'control operation rejected: {words}: {reply}; run={run}')
        return reply

    def wait_marker(marker, proc, timeout=120):
        deadline = time.monotonic() + timeout
        while marker not in game_log.read_text(encoding='utf-8', errors='replace').splitlines():
            if proc.poll() is not None or time.monotonic() > deadline:
                raise RuntimeError(f'missing {marker!r}: {game_log}')
            time.sleep(.1)

    def arrange_scene():
        for command in (
            'clear @s', 'time set 6000', 'weather clear',
            'fill -4 99 -4 10 99 10 minecraft:stone',
            'fill -4 100 -4 10 110 10 minecraft:air',
            'tp @s 3.5 101 3.5',
        ):
            ctl('command', command)
        ctl('wait', '10')
        ctl('look-at', 'block', '3', '99', '0')
        ctl('key', 'f1', 'tap')
        ctl('wait', '40')

    def save_screenshot(path):
        ctl('screenshot', str(path))
        if not path.is_file() or path.stat().st_size == 0:
            raise RuntimeError(f'the screenshot was not saved: {path}')
        screenshots.append(str(path))

    screenshots = []
    screenshot = run / 'client-resources.png'
    with game_log.open('w', encoding='utf-8', newline='\n') as output:
        proc = subprocess.Popen([str(game), '--resources', str(resources), '--control-socket', str(endpoint), '--offscreen', '--world', 'ClientResourcesProbe', '--seed', '42', '--creative', '--test-terrain', '--render-distance', '2', '--width', '1280', '--height', '720'], cwd=run, env=env, stdout=output, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic() + 120
            while not endpoint.exists():
                if proc.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError(f'game did not start: {game_log}')
                time.sleep(.2)
            ctl('wait-ready', '120', timeout=125)
            wait_marker(EXPECTED[0], proc)
            # These checks run inside the consumer before each pass=true marker:
            # known raw resource bytes, decoded image dimensions/RGBA length, and missing None.
            if args.pack_override:
                wait_marker(PACK_EXPECTED[0], proc)
                arrange_scene()
                save_screenshot(run / 'initial-client-resources.png')
                # Leave the lower pack unchanged; both reads must refresh to the higher
                # pack's new bytes and exact top-first, non-premultiplied RGBA pixels.
                write_pack_assets(high_pack, b'reloaded', RELOADED_PIXELS)
                write_stone_texture(high_pack, [1, 180, 200, 255])
            ctl('key', 'f3', 'down')
            try:
                ctl('key', 't', 'tap', timeout=125)
            finally:
                if proc.poll() is None:
                    ctl('key', 'f3', 'up', timeout=125)
            wait_marker(EXPECTED[1], proc)
            if args.pack_override:
                wait_marker(PACK_EXPECTED[1], proc)
            ctl('wait-ready', '120', timeout=125)
            ctl('close')
            ctl('wait', '5')
            if ctl('status')['debug']['overlay']:
                ctl('key', 'f3', 'tap')
                ctl('wait', '5')
            status = ctl('status')
            if status['screen'] != 'playing' or status['debug']['overlay']:
                raise RuntimeError(f'UI was not reset after reload: {status}; run={run}')
            if not args.pack_override:
                arrange_scene()
            else:
                # No movement, camera changes or F1 toggle between the two captures.
                ctl('wait', '40')
            save_screenshot(screenshot)
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
    callbacks = [line for line in lines if line.startswith('PLUTO-PROBE client_resources count=')]
    if callbacks != EXPECTED:
        raise RuntimeError(f'expected exactly initial/reload pass markers, got {callbacks}; log={game_log}')
    pack_callbacks = [line for line in lines if line.startswith('PLUTO-PROBE client_pack count=')]
    if pack_callbacks != (PACK_EXPECTED if args.pack_override else []):
        raise RuntimeError(f'unexpected pack override callbacks: {pack_callbacks}; log={game_log}')
    joined = next((index for index, line in enumerate(lines) if line.startswith('PLUTO-PROBE join name=')), None)
    initial = [EXPECTED[0], PACK_EXPECTED[0]] if args.pack_override else [EXPECTED[0]]
    if joined is None or any(lines.index(marker) >= joined for marker in initial):
        raise RuntimeError(f'initial client resources must be ready before player join: {game_log}')
    lifecycle = [f'PLUTO-PROBE server_{phase} dedicated=false' for phase in ('starting', 'stopping', 'stopped')]
    if any(lines.count(event) != 1 for event in lifecycle):
        raise RuntimeError(f'lifecycle events must occur exactly once: {game_log}')
    if [lines.index(event) for event in lifecycle] != sorted(lines.index(event) for event in lifecycle):
        raise RuntimeError(f'lifecycle events are out of order: {game_log}')
    print(json.dumps({'result': 'CLIENT RESOURCES PROBE PASS', 'run': str(run), 'screenshots': screenshots, 'callbacks': callbacks, 'pack_callbacks': pack_callbacks, 'pack_override_tested': args.pack_override, 'image_review_required': True}, indent=2))


if __name__ == '__main__':
    main()
