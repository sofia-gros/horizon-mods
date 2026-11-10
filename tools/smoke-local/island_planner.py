#!/usr/bin/env python3
"""Place the unmodified production island settings through the real planner.

Explicit commands deliberately use native biome->true policy, even in the
Overworld. End placement is not natural generation or saved-start coverage.
Runtime templates still omit the custom fluid. Inspect both saved images.
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
    for name in ('game', 'ctl', 'resources'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    game, controller, resources = args.game.resolve(), args.ctl.resolve(), args.resources.resolve()
    pins = json.loads((ROOT / 'tconstruct/compatibility.json').read_text(encoding='utf-8'))
    require_report(game.parent, pins['horizon_revision'], pins['mixins_revision'],
                   {'pluto_lib', 'pluto_probe', 'tconstruct', 'pluto_native_creative'})
    catalog = json.loads((resources / 'worldgen/structures.json').read_text(encoding='utf-8'))
    settings = catalog['tconstruct:end_slime_island']
    if isinstance(settings, str):
        settings = json.loads(settings)
    original = json.loads((ROOT / 'tconstruct/content/reference/slime-islands/data/tconstruct/worldgen/structure/end_slime_island.json').read_text(encoding='utf-8'))
    if settings != original:
        raise RuntimeError('prepared island settings differ from the original reference')
    runs = ROOT / '.local/smoke-runs'
    runs.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix='island-planner-', dir=runs)).resolve()
    endpoint, log = run / 'control.json', run / 'game.log'
    env = dict(os.environ, HORIZON_NO_AUDIO='1', HORIZON_NO_DISCORD='1', HORIZON_TELEMETRY_URL='off',
               HORIZON_LAUNCHER_NO_SELF_UPDATE='1', RUST_MIN_STACK='16777216', PLUTO_PROBE_STRUCTURE_MODE='1')
    for key in ('PLUTO_PROBE_FEATURE_MODE', 'PLUTO_PROBE_RESOURCE_PACK'):
        env.pop(key, None)
    for key in ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'TMP', 'TEMP'):
        folder = run / key.lower()
        folder.mkdir()
        env[key] = str(folder)
    observations, screenshots = [], []

    def ctl(*words, timeout=60):
        words = list(map(str, words))
        reply = subprocess.run([str(controller), '--socket', str(endpoint), '--timeout', str(timeout), *words],
                               capture_output=True, text=True, timeout=timeout + 5)
        with (run / 'ctl.jsonl').open('a', encoding='utf-8', newline='\n') as output:
            output.write(json.dumps({'args': words, 'code': reply.returncode,
                                     'stdout': reply.stdout, 'stderr': reply.stderr}) + '\n')
        if reply.returncode:
            raise RuntimeError(f'control failed {words}: {reply.stdout} {reply.stderr}; run={run}')
        result = json.loads(reply.stdout)
        if result.get('error') or result.get('ok') is False:
            raise RuntimeError(f'control rejected {words}: {result}; run={run}')
        return result

    def ground():
        # The original template palettes can choose either dirt. Read the real
        # client after network updates, not immediately in the command frame.
        ctl('wait', 40)
        for block in ('tconstruct:ender_slime_dirt', 'tconstruct:ichor_slime_dirt'):
            found = ctl('find-block', block, 64)
            if found.get('count', 0) > 0:
                return found
        raise RuntimeError(f'command reported success without observed island ground; run={run}')

    def photo(name, found):
        x, y, z = found['nearest'][0]['pos']
        ctl('command', f'tp @s {x+24.5} {y+25} {z+24.5}')
        ctl('look-at', x, y+6, z)
        ctl('wait', 80)
        ctl('key', 'f1', 'tap')
        try:
            path = run / (name + '.png')
            ctl('screenshot', path)
            if not path.is_file() or not path.stat().st_size:
                raise RuntimeError(f'missing screenshot: {path}')
            screenshots.append(str(path))
        finally:
            ctl('key', 'f1', 'tap')

    def generated(reply):
        return any('Generated structure' in line for line in reply.get('chat', []))

    with log.open('w', encoding='utf-8', newline='\n') as output:
        proc = subprocess.Popen([str(game), '--resources', str(resources), '--control-socket', str(endpoint),
            '--offscreen', '--world', 'IslandPlanner', '--seed', '42', '--creative', '--render-distance', '4',
            '--width', '1280', '--height', '720'], cwd=run, env=env, stdout=output, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic() + 120
            while not endpoint.exists():
                if proc.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError(f'game did not start; run={run}')
                time.sleep(.2)
            ctl('wait-ready', 120, timeout=125)
            for command in ('gamemode spectator', 'gamerule random_tick_speed 0', 'time set 6000',
                            'weather clear', 'tp @s 0.5 160 0.5'):
                ctl('command', command)
            ctl('wait', 80)
            reply = ctl('command', 'place structure tconstruct:end_slime_island 0 160 0')
            if not generated(reply):
                raise RuntimeError(f'production explicit placement failed in Overworld: {reply}; run={run}')
            found = ground()
            observations.append({'dimension': 'minecraft:overworld', 'reply': reply, 'ground': found,
                'biome_policy': 'explicit native command uses biome->true, not natural biome acceptance'})
            photo('production-overworld', found)
            for x, z in ((2304, 2048), (2048, 2048), (2048, 2304)):
                ctl('command', f'execute in minecraft:the_end run tp @s {x+.5} 160 {z+.5}')
                ctl('wait', 120)
                state = ctl('state')
                if state.get('dimension') != 'minecraft:the_end':
                    raise RuntimeError(f'dimension transfer failed: {state.get("dimension")}; run={run}')
                loading = []
                for _ in range(4):
                    reply = ctl('command', f'place structure tconstruct:end_slime_island {x} 160 {z}')
                    loading.append(reply)
                    if not any('That position is not loaded' in line for line in reply.get('chat', [])):
                        break
                    ctl('wait', 120)
                if not generated(reply):
                    observations.append({'dimension': state['dimension'], 'target': [x,160,z],
                                         'attempts': loading, 'placed': False})
                    # Only loading is retriable. A placement/callback error is
                    # not masked by another successful fixture.
                    if any('That position is not loaded' in line for line in reply.get('chat', [])):
                        continue
                    raise RuntimeError(f'production End placement failed: {reply}; run={run}')
                found = ground()
                observations.append({'dimension': state['dimension'], 'target': [x,160,z],
                                     'attempts': loading, 'ground': found, 'placed': True})
                photo('production-end', found)
                break
            else:
                raise RuntimeError(f'no loaded production End placement after bounded waits; run={run}')
            ctl('quit')
            proc.wait(timeout=60)
            if proc.returncode:
                raise RuntimeError(f'game exited {proc.returncode}; run={run}')
        finally:
            if proc.poll() is None:
                proc.terminate()
                try:
                    proc.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait(timeout=15)
    text = log.read_text(encoding='utf-8', errors='replace')
    for phase in ('starting', 'stopping', 'stopped'):
        if text.count(f'PLUTO-PROBE server_{phase} dedicated=false') != 1:
            raise RuntimeError(f'unclean lifecycle {phase}; run={run}')
    result = {'result': 'ISLAND PLANNER PASS', 'run': str(run), 'observations': observations,
              'screenshots': screenshots, 'scope': 'production explicit planning only; custom fluid, natural candidates/biomes/cache and saved-start reloads unverified'}
    (run / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8', newline='\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
