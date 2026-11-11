#!/usr/bin/env python3
"""Generate original End islands and marker features, without /place commands.

Only the isolated clone's candidate set is made dense and deterministic. The
production structure configuration and biome tag remain unchanged. This checks
natural dispatch/markers, not original placement frequency or custom fluids.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

from native_build_report import require_report
from structures import ROOT, prepare_resources


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('game', 'ctl', 'resources'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    game, controller, original = args.game.resolve(), args.ctl.resolve(), args.resources.resolve()
    pins = json.loads((ROOT / 'tconstruct/compatibility.json').read_text(encoding='utf-8'))
    require_report(game.parent, pins['horizon_revision'], pins['mixins_revision'],
                   {'pluto_lib', 'pluto_probe', 'tconstruct', 'pluto_native_creative'})
    run = Path(tempfile.mkdtemp(prefix='natural-islands-', dir=ROOT / '.local/smoke-runs')).resolve()
    resources, _ = prepare_resources(original, run)
    catalog = json.loads((resources / 'worldgen/structures.json').read_text(encoding='utf-8'))
    settings = catalog['tconstruct:end_slime_island']
    settings = json.loads(settings) if isinstance(settings, str) else settings
    reference = json.loads((ROOT / 'tconstruct/content/reference/slime-islands/data/tconstruct/worldgen/structure/end_slime_island.json').read_text(encoding='utf-8'))
    if settings != reference:
        raise RuntimeError('natural island settings differ from original source reference')
    path = resources / 'worldgen/structure_sets.json'
    before = path.read_bytes()
    sets = json.loads(before)
    # Avoid a duplicate start of the same production structure in our fixture.
    sets.pop('tconstruct:end_sky_island', None)
    sets['pluto_probe:natural_end_island'] = json.dumps({
        'structures': [{'structure': 'tconstruct:end_slime_island', 'weight': 1}],
        'placement': {'type': 'minecraft:random_spread', 'spacing': 32, 'separation': 31, 'salt': 1731}})
    path.unlink()
    path.write_text(json.dumps(sets) + '\n', encoding='utf-8', newline='\n')
    if (original / 'worldgen/structure_sets.json').read_bytes() != before:
        raise RuntimeError('original structure sets changed')
    endpoint = run / 'control.json'
    env = dict(os.environ, HORIZON_NO_AUDIO='1', HORIZON_NO_DISCORD='1', HORIZON_TELEMETRY_URL='off',
               HORIZON_LAUNCHER_NO_SELF_UPDATE='1', RUST_MIN_STACK='16777216', PLUTO_PROBE_STRUCTURE_MODE='1')
    for name in ('PLUTO_PROBE_FEATURE_MODE', 'PLUTO_PROBE_RESOURCE_PACK', 'PLUTO_PROBE_NATURAL_MODE', 'PLUTO_PROBE_NATURAL_RESUME'):
        env.pop(name, None)
    for name in ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'TMP', 'TEMP'):
        directory = run / name.lower()
        directory.mkdir()
        env[name] = str(directory)

    def ctl(*words, timeout=60):
        words = list(map(str, words))
        reply = subprocess.run([str(controller), '--socket', str(endpoint), '--timeout', str(timeout), *words],
                               capture_output=True, text=True, timeout=timeout+5)
        with (run / 'ctl.jsonl').open('a', encoding='utf-8', newline='\n') as output:
            output.write(json.dumps({'args': words, 'code': reply.returncode, 'stdout': reply.stdout, 'stderr': reply.stderr}) + '\n')
        if reply.returncode:
            raise RuntimeError(f'control failed {words}: {reply.stdout} {reply.stderr}; run={run}')
        result = json.loads(reply.stdout)
        if result.get('error') or result.get('ok') is False:
            raise RuntimeError(f'control rejected {words}: {result}; run={run}')
        return result

    observations, screenshots = [], []
    log = run / 'game.log'
    with log.open('w', encoding='utf-8', newline='\n') as output:
        proc = subprocess.Popen([str(game), '--resources', str(resources), '--control-socket', str(endpoint),
            '--offscreen', '--world', 'NaturalIslands', '--seed', '42', '--creative',
            '--render-distance', '4', '--width', '1280', '--height', '720'], cwd=run, env=env,
            stdout=output, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic()+120
            while not endpoint.exists():
                if proc.poll() is not None or time.monotonic()>deadline:
                    raise RuntimeError(f'game did not start; run={run}')
                time.sleep(.2)
            ctl('wait-ready', 120, timeout=125)
            for command in ('gamemode spectator', 'gamerule random_tick_speed 0', 'time set 6000', 'weather clear'):
                ctl('command', command)
            # Every anchor is the potential chunk's corner of a separate native
            # spacing cell. All are outside the central End biome.
            for index, (x,z) in enumerate(((2048,2048),(2560,2048),(2048,2560),(2560,2560),(3072,2048),(2048,3072))):
                ctl('command', f'execute in minecraft:the_end run tp @s {x+8} 160 {z+8}')
                ctl('wait', 200)
                ground = []
                # The controller caps radius at 64; SKY islands over void can
                # be below y=96. Survey overlapping vertical bands, not just 160.
                for altitude in (160, 80, 224):
                    if altitude != 160:
                        ctl('command', f'tp @s {x+8} {altitude} {z+8}')
                        ctl('wait', 80)
                    for name in ('tconstruct:ender_slime_dirt', 'tconstruct:ichor_slime_dirt'):
                        found = ctl('find-block', name, 64)
                        if found.get('count',0)>0:
                            ground.append(found)
                    if ground:
                        break
                if not ground:
                    raise RuntimeError(f'natural original island absent across height bands at candidate {x,z}; run={run}')
                trees = ctl('find-block', 'tconstruct:enderbark_log', 64)
                leaves = ctl('find-block', 'tconstruct:ender_slime_leaves', 64)
                plants = {name: ctl('find-block', name, 64).get('count',0) for name in
                          ('tconstruct:ender_slime_tall_grass','tconstruct:ender_slime_fern','tconstruct:ender_slime_vine')}
                position = ground[0]['nearest'][0]['pos']
                ctl('command', f'tp @s {position[0]+28} {position[1]+18} {position[2]+28}')
                ctl('look-at', position[0],position[1]+3,position[2])
                ctl('wait',40)
                ctl('key','f1','tap')
                try:
                    image = run / f'island-{index}.png'
                    ctl('screenshot',image)
                    if not image.is_file() or not image.stat().st_size:
                        raise RuntimeError('missing natural island screenshot')
                    screenshots.append(str(image))
                finally:
                    ctl('key','f1','tap')
                observations.append({'candidate':[x,z], 'ground':ground,'trees':trees.get('count',0),
                                     'leaves':leaves.get('count',0),'plants':plants})
                if (any(row['trees']>0 and row['leaves']>0 for row in observations)
                    and all(any(row['plants'][name]>0 for row in observations) for name in
                            ('tconstruct:ender_slime_tall_grass','tconstruct:ender_slime_fern','tconstruct:ender_slime_vine'))):
                    break
            else:
                raise RuntimeError(f'missing original tree, tall grass, fern or vine markers across six natural islands; run={run}')
            located = ctl('command','locate structure tconstruct:end_slime_island')
            if not any('nearest' in text.lower() and 'tconstruct:end_slime_island' in text for text in located.get('chat',[])):
                raise RuntimeError(f'natural island locate did not find a generated start: {located}; run={run}')
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
    text = log.read_text(encoding='utf-8',errors='replace')
    for status in ('starting','stopping','stopped'):
        if text.count(f'PLUTO-PROBE server_{status} dedicated=false')!=1:
            raise RuntimeError(f'unclean lifecycle {status}; run={run}')
    for failure in ('registered piece requires the typed placement adapter', 'registered structure state invalid',
                    'natural structure tconstruct:end_slime_island rejected', 'tree configuration failed', 'tree placement failed'):
        if failure in text:
            raise RuntimeError(f'natural marker dispatch failure {failure}; run={run}')
    result = {'result':'NATURAL ISLANDS PASS','run':str(run),'observations':observations,'screenshots':screenshots,
              'scope':'original structure settings on deterministic fixture candidates; distribution/custom-fluid/LAN parity not checked'}
    (run/'result.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8',newline='\n')
    print(json.dumps(result,indent=2))


if __name__=='__main__':
    main()
