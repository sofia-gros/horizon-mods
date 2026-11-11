#!/usr/bin/env python3
"""Throw real snowballs at production crystals and require both Java sound events.

Audio output stays disabled. This verifies emitted event counts, not audible
playback, exact native RNG traces, LAN or other dimensions.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

from native_build_report import require_report
from structures import ROOT


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('game', 'ctl', 'resources'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    game, controller, resources = (getattr(args, name).resolve() for name in ('game', 'ctl', 'resources'))
    pins = json.loads((ROOT / 'tconstruct/compatibility.json').read_text(encoding='utf-8'))
    require_report(game.parent, pins['horizon_revision'], pins['mixins_revision'],
                   {'pluto_lib', 'pluto_probe', 'tconstruct', 'pluto_native_creative'})
    run = Path(tempfile.mkdtemp(prefix='crystal-chimes-', dir=ROOT / '.local/smoke-runs')).resolve()
    endpoint = run / 'control.json'
    env = dict(os.environ, HORIZON_NO_AUDIO='1', HORIZON_NO_DISCORD='1', HORIZON_TELEMETRY_URL='off',
               HORIZON_LAUNCHER_NO_SELF_UPDATE='1', PLUTO_PROBE_STRUCTURE_MODE='1', RUST_MIN_STACK='16777216')
    for key in ('PLUTO_PROBE_FEATURE_MODE', 'PLUTO_PROBE_NATURAL_MODE', 'PLUTO_PROBE_NATURAL_RESUME', 'PLUTO_PROBE_RESOURCE_PACK'):
        env.pop(key, None)
    for key in ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'TMP', 'TEMP'):
        folder = run / key.lower()
        folder.mkdir()
        env[key] = str(folder)
    cases = []
    for kind in ('earth', 'sky', 'ichor', 'ender'):
        chime = f'tconstruct:block.{kind}_crystal.chime'
        cases.append((f'tconstruct:{kind}_slime_crystal_block', 'minecraft:block.amethyst_block.hit', chime))
        cases.append((f'tconstruct:{kind}_slime_crystal_cluster[facing=up,waterlogged=false]', 'minecraft:block.amethyst_cluster.hit', chime))
    for kind, material, chime in (('steel', 'metal', 'sky'), ('cobalt', 'netherite_block', 'ichor'), ('knightmetal', 'netherite_block', 'ender')):
        cases.append((f'tconstruct:{kind}_cluster[facing=up,waterlogged=false]', f'minecraft:block.{material}.hit', f'tconstruct:block.{chime}_crystal.chime'))
    cases.append(('minecraft:stone', None, None))
    expected_events = {event for _, hit, chime in cases for event in (hit, chime) if event is not None}
    rows, screenshots, failures = [], [], []

    def ctl(*words):
        reply = subprocess.run([str(controller), '--socket', str(endpoint), '--timeout', '60', *map(str, words)],
                               capture_output=True, text=True, timeout=65)
        with (run / 'ctl.jsonl').open('a', encoding='utf-8', newline='\n') as output:
            output.write(json.dumps({'args': list(map(str, words)), 'code': reply.returncode, 'stdout': reply.stdout, 'stderr': reply.stderr}) + '\n')
        result = json.loads(reply.stdout)
        if reply.returncode or result.get('error') or result.get('ok') is False:
            raise RuntimeError(f'control rejected {words}: {result}; run={run}')
        return result

    log = run / 'game.log'
    with log.open('w', encoding='utf-8', newline='\n') as output:
        proc = subprocess.Popen([str(game), '--resources', str(resources), '--control-socket', str(endpoint),
            '--offscreen', '--world', 'CrystalChimes', '--seed', '42', '--creative', '--test-terrain',
            '--render-distance', '2', '--width', '1280', '--height', '720'], cwd=run, env=env,
            stdout=output, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic() + 120
            while not endpoint.exists():
                if proc.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError(f'game did not start; run={run}')
                time.sleep(.2)
            ctl('wait-ready', 120)
            for command in ('clear @s', 'give @s minecraft:snowball 64', 'gamerule random_tick_speed 0',
                            'time set 6000', 'weather clear', 'fill -3 99 -3 3 99 6 minecraft:stone',
                            'fill -3 100 -3 3 108 6 minecraft:air', 'setblock 0 100 4 minecraft:stone',
                            'setblock 0 102 0 minecraft:stone'):
                ctl('command', command)
            for index, (state, hit, chime) in enumerate(cases):
                ctl('command', 'gamemode creative')
                ctl('setblock', 0, 103, 0, state)
                ctl('command', 'tp @s 0.5 101 4.5')
                ctl('wait', 10)
                ctl('select', 'minecraft:snowball')
                before = ctl('status')['heard']
                for _ in range(2):
                    ctl('look-at-block', 0, 103, 0)
                    ctl('use-item')
                    ctl('wait', 40)
                status = ctl('status')
                after = status['heard']
                delta = {event: after.get(event, 0) - before.get(event, 0) for event in expected_events}
                expected = {event: 2 if event in (hit, chime) else 0 for event in expected_events}
                if delta != expected:
                    failures.append({'block': state, 'expected': expected, 'actual': delta})
                throws = after.get('minecraft:entity.snowball.throw', 0) - before.get('minecraft:entity.snowball.throw', 0)
                if throws != 2 or status['projectiles']:
                    failures.append({'block': state, 'throws': throws, 'remaining_projectiles': status['projectiles']})
                actual = ctl('getblock', 0, 103, 0)['block']
                if actual.split('[', 1)[0] != state.split('[', 1)[0]:
                    failures.append({'block': state, 'changed_to': actual})
                rows.append({'block': state, 'throws': throws, 'events': {key: value for key, value in delta.items() if value}})
                # One image per crystal kind and metal cluster, no forced ray or
                # projectile hit. Spectator mode is only used after the throws.
                if index % 2 == 1 or index >= 8:
                    ctl('command', 'gamemode spectator')
                    ctl('command', 'tp @s 2.5 103 4.5')
                    ctl('look-at', .5, 103.3, .5)
                    ctl('wait', 20)
                    ctl('key', 'f1', 'tap')
                    try:
                        image = run / f'crystal-{index}.png'
                        ctl('screenshot', image)
                        if not image.is_file() or not image.stat().st_size:
                            raise RuntimeError(f'missing image {image}')
                        screenshots.append(str(image))
                    finally:
                        ctl('key', 'f1', 'tap')
            ctl('quit')
            proc.wait(timeout=60)
            if proc.returncode:
                failures.append({'exit_code': proc.returncode})
        finally:
            if proc.poll() is None:
                proc.terminate()
                try:
                    proc.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait(timeout=15)
    text = log.read_text(encoding='utf-8', errors='replace')
    for event in ('starting', 'stopping', 'stopped'):
        if text.count(f'PLUTO-PROBE server_{event} dedicated=false') != 1:
            failures.append({'lifecycle': event})
    result = {'result': 'CRYSTAL CHIMES FAIL' if failures else 'CRYSTAL CHIMES PASS', 'run': str(run),
              'observations': rows, 'failures': failures, 'screenshots': screenshots,
              'scope': 'authoritative projectile sound-event emission; audible playback, native RNG trace, LAN and other dimensions not proved'}
    (run / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8', newline='\n')
    print(json.dumps(result, indent=2))
    if failures:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
