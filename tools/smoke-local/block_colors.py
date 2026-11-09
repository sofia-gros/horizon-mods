#!/usr/bin/env python3
"""Focused world-mesh color probe; screenshots still require a human/agent image review."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
HORIZON = 'ebcfa2263aa3d8e399aa993b1e05daf42eb4cfa0'
MIXINS = '5a64af938ec86b00d53d16a408aabc0038fb3e57'
COLOR_EVENT = 'PLUTO-PROBE block_color pos=[0, 101, 0] tint=0 persistent=Some("true")'
FALLBACK_EVENT = 'PLUTO-PROBE block_color fallback pos=[3, 101, 0]'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--game', type=Path, required=True)
    parser.add_argument('--ctl', type=Path, required=True)
    parser.add_argument('--resources', type=Path, required=True)
    args = parser.parse_args()
    game, controller, resources = args.game.resolve(), args.ctl.resolve(), args.resources.resolve()
    if not game.is_file() or not controller.is_file() or not resources.is_dir():
        parser.error('--game/--ctl must be files and --resources must be a prepared resource directory')
    report = json.loads((game.parent / 'native-mods.json').read_text(encoding='utf-8'))
    if report.get('horizon') != HORIZON or report.get('horizon_mixins') != MIXINS:
        raise RuntimeError('block color gate requires the reviewed Horizon/Mixins pins')
    built = {mod['id'] for mod in report.get('mods', [])}
    if report.get('left_out') != [] or not {'pluto_lib', 'pluto_probe', 'tconstruct'} <= built:
        raise RuntimeError('required native mods must all be included with left_out=[]')

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

    screenshot = run / 'colors.png'
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
                'tp @s 3.5 101 8.5',
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
            ctl('key', 'f1', 'tap')
            ctl('wait', '5')
            ctl('screenshot', str(screenshot))
            if not screenshot.is_file() or screenshot.stat().st_size == 0:
                raise RuntimeError(f'the screenshot was not saved: {screenshot}')
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
    print(json.dumps({'result': 'BLOCK COLORS PROBE PASS', 'run': str(run), 'screenshots': [str(screenshot)], 'callbacks': [COLOR_EVENT, FALLBACK_EVENT], 'image_review_required': True}, indent=2))


if __name__ == '__main__':
    main()
