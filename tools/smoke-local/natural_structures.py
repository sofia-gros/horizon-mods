#!/usr/bin/env python3
"""Verify natural plain-template starts, native fallback and saved-start resume.

No explicit placement commands. Registered marker pieces and custom sets with
avoid-biome policy remain unsupported. This does not enable natural islands.
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


def prepare(original, run):
    resources, _ = prepare_resources(original, run)
    path = resources / 'worldgen/structures.json'
    definitions = json.loads(path.read_text(encoding='utf-8'))
    hut = definitions['minecraft:swamp_hut']
    hut = json.loads(hut) if isinstance(hut, str) else hut
    added = []
    cases = [('plain', 8), ('reject_plain', 8), ('reject_hut', 8), ('bad_biome', 24),
             ('marker', 40), ('avoid', 56)]
    for case, x in cases:
        name = 'pluto_probe:natural_' + case
        if name in definitions:
            raise RuntimeError('fixture structure exists: ' + name)
        definitions[name] = json.dumps({'type': 'pluto_probe:natural_structure',
            'biomes': ['minecraft:desert'] if case == 'bad_biome' else '#minecraft:is_overworld',
            'step': 'surface_structures', 'spawn_overrides': {}, 'terrain_adaptation': 'none',
            'probe_case': case, 'probe_origin': [x, 110, 8]})
        added.append(name)
    for case in ('native_hut', 'native_avoid'):
        name = 'pluto_probe:natural_' + case
        if name in definitions:
            raise RuntimeError('native fixture exists: ' + name)
        value = dict(hut, biomes='#minecraft:is_overworld', terrain_adaptation='none')
        definitions[name] = json.dumps(value)
        added.append(name)
    path.write_text(json.dumps(definitions) + '\n', encoding='utf-8', newline='\n')
    path = resources / 'worldgen/structure_sets.json'
    before = path.read_bytes()
    sets = json.loads(before)
    groups = [('plain', [('reject_plain', 1000000), ('plain', 1)]),
              ('native', [('reject_hut', 1000000), ('native_hut', 1)]),
              ('bad_biome', [('bad_biome', 1)]), ('marker', [('marker', 1)]),
              ('avoid', [('avoid', 1)]), ('native_avoid', [('native_avoid', 1)])]
    for case, entries in groups:
        name = 'pluto_probe:natural_' + case
        if name in sets:
            raise RuntimeError('fixture set exists: ' + name)
        # Native nextInt(spacing-separation) has bound one: candidate [0,0]
        # near spawn, not a fixture on every chunk. No copied RNG model.
        placement = {'type': 'minecraft:random_spread', 'spacing': 32, 'separation': 31, 'salt': 1731}
        if case in ('avoid', 'native_avoid'):
            placement['horizon:avoid_biomes'] = {'biomes': ['#minecraft:is_overworld'], 'radius': 0, 'step': 1, 'y': 120}
        sets[name] = json.dumps({'structures': [{'structure': 'pluto_probe:natural_' + n, 'weight': w} for n, w in entries],
                                'placement': placement})
    path.unlink()  # detach the clone's hard link
    path.write_text(json.dumps(sets) + '\n', encoding='utf-8', newline='\n')
    if (original / 'worldgen/structure_sets.json').read_bytes() != before:
        raise RuntimeError('original structure sets changed')
    path = resources / 'command-registries.json'  # already detached
    registry = json.loads(path.read_text(encoding='utf-8'))
    ids = registry['by_registry']['minecraft:worldgen/structure']['ids']
    for name in added:
        if name in ids:
            raise RuntimeError('fixture command exists: ' + name)
        ids.append(name)
    path.write_text(json.dumps(registry) + '\n', encoding='utf-8', newline='\n')
    return resources


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ('game', 'ctl', 'resources'):
        parser.add_argument('--' + key, type=Path, required=True)
    args = parser.parse_args()
    game, controller, original = args.game.resolve(), args.ctl.resolve(), args.resources.resolve()
    pins = json.loads((ROOT / 'tconstruct/compatibility.json').read_text(encoding='utf-8'))
    require_report(game.parent, pins['horizon_revision'], pins['mixins_revision'],
                   {'pluto_lib', 'pluto_probe', 'tconstruct', 'pluto_native_creative'})
    run = Path(tempfile.mkdtemp(prefix='natural-structures-', dir=ROOT / '.local/smoke-runs')).resolve()
    resources = prepare(original, run)
    endpoint = run / 'control.json'
    env = dict(os.environ, HORIZON_NO_AUDIO='1', HORIZON_NO_DISCORD='1', HORIZON_TELEMETRY_URL='off',
               HORIZON_LAUNCHER_NO_SELF_UPDATE='1', RUST_MIN_STACK='16777216',
               PLUTO_PROBE_STRUCTURE_MODE='1', PLUTO_PROBE_NATURAL_MODE='1')
    for key in ('PLUTO_PROBE_FEATURE_MODE', 'PLUTO_PROBE_RESOURCE_PACK', 'PLUTO_PROBE_NATURAL_RESUME'):
        env.pop(key, None)
    for key in ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'TMP', 'TEMP'):
        folder = run / key.lower()
        folder.mkdir()
        env[key] = str(folder)
    observations, screenshots = [], []
    phase = 'fresh'

    def ctl(*words, timeout=60):
        words = list(map(str, words))
        reply = subprocess.run([str(controller), '--socket', str(endpoint), '--timeout', str(timeout), *words],
                               capture_output=True, text=True, timeout=timeout+5)
        with (run / 'ctl.jsonl').open('a', encoding='utf-8', newline='\n') as output:
            output.write(json.dumps({'phase': phase, 'args': words, 'code': reply.returncode,
                                     'stdout': reply.stdout, 'stderr': reply.stderr}) + '\n')
        if reply.returncode:
            raise RuntimeError(f'control failed {words}: {reply.stdout} {reply.stderr}; run={run}')
        result = json.loads(reply.stdout)
        if result.get('error') or result.get('ok') is False:
            raise RuntimeError(f'control rejected {words}: {result}; run={run}')
        return result

    def locate(case, wanted):
        name = 'pluto_probe:natural_' + case
        reply = ctl('command', 'locate structure ' + name, timeout=120)
        found = any('nearest' in line.lower() and name in line for line in reply.get('chat', []))
        if found != wanted:
            raise RuntimeError(f'{phase}: wrong locate result for {case}: {reply}; run={run}')
        return reply

    for phase in ('fresh', 'resume'):
        endpoint.unlink(missing_ok=True)  # previous process has fully exited
        current_env = dict(env)
        if phase == 'resume':
            current_env['PLUTO_PROBE_NATURAL_RESUME'] = '1'
        log = run / (phase + '.log')
        with log.open('w', encoding='utf-8', newline='\n') as output:
            proc = subprocess.Popen([str(game), '--resources', str(resources), '--control-socket', str(endpoint),
                '--offscreen', '--world', 'NaturalStructures', '--seed', '42', '--creative',
                '--render-distance', '4', '--width', '1280', '--height', '720'], cwd=run, env=current_env,
                stdout=output, stderr=subprocess.STDOUT)
            try:
                deadline = time.monotonic()+120
                while not endpoint.exists():
                    if proc.poll() is not None or time.monotonic()>deadline:
                        raise RuntimeError(f'{phase}: game did not start; run={run}')
                    time.sleep(.2)
                ctl('wait-ready', 120, timeout=125)
                for command in ('gamemode spectator', 'gamerule random_tick_speed 0', 'time set 6000',
                                'weather clear', 'tp @s 8.5 115 16.5'):
                    ctl('command', command)
                ctl('wait', 120)
                points = [(8,110,8,'gold_block'), (10,110,8,'diamond_block'), (8,110,9,'emerald_block'),
                          (10,110,9,'lapis_block'), (9,111,8,'oak_stairs'), (9,110,8,'air')]
                states = []
                for x,y,z,block in points:
                    state = ctl('getblock', x,y,z)['block']
                    if state.split('[',1)[0] != 'minecraft:' + block:
                        raise RuntimeError(f'{phase}: missing natural template block at {x,y,z}: {state}; run={run}')
                    states.append(state)
                for x in (24,40,56):
                    if ctl('getblock', x,110,8)['block'].split('[',1)[0]=='minecraft:gold_block':
                        raise RuntimeError(f'{phase}: rejected natural piece wrote at {x}; run={run}')
                replies = {'plain': locate('plain', True), 'native_hut': locate('native_hut', True)}
                # Negative /locate searches exhaust the native command's wide
                # search radius and may exceed its server RPC budget. Rejected
                # fixture writes and actual dispatch logs are checked instead;
                # this does not claim runtime census of every rejected cache.
                planks = ctl('find-block', 'minecraft:spruce_planks', 64)
                if planks.get('count',0)<=0:
                    raise RuntimeError(f'{phase}: native weighted fallback hut not observed; run={run}')
                ctl('look-at', 9,110.5,8.5)
                ctl('wait',40)
                ctl('key','f1','tap')
                try:
                    image = run / (phase+'.png')
                    ctl('screenshot',image)
                    if not image.is_file() or not image.stat().st_size:
                        raise RuntimeError('missing screenshot')
                    screenshots.append(str(image))
                finally:
                    ctl('key','f1','tap')
                observations.append({'phase':phase,'states':states,'locate':replies,'native_planks':planks})
                ctl('quit')
                proc.wait(timeout=60)
                if proc.returncode:
                    raise RuntimeError(f'{phase}: game exited {proc.returncode}; run={run}')
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
                raise RuntimeError(f'{phase}: unclean lifecycle {status}; run={run}')
        if 'PLUTO-PROBE forbidden built-in structure override' in text or 'PLUTO-PROBE marker case=natural_marker' in text:
            raise RuntimeError(f'{phase}: unsupported/built-in dispatch leaked; run={run}')
        if phase=='fresh':
            for expected in ('natural planned case=plain', 'natural declined case=reject_plain reason=consumer',
                             'natural declined case=reject_hut reason=consumer'):
                if expected not in text:
                    raise RuntimeError(f'fresh: missing actual weighted dispatch {expected}; run={run}')
        else:
            if 'natural unexpected_replan' in text:
                raise RuntimeError(f'resume regenerated a saved start; run={run}')
            if 'pluto_lib: restored saved structure piece kind=pluto_lib:template_piece' not in text:
                raise RuntimeError(f'resume did not observe successful native piece restoration; run={run}')
    result = {'result':'NATURAL STRUCTURES PASS','run':str(run),'observations':observations,'screenshots':screenshots,
              'scope':'plain-template natural starts only; natural markers/islands and custom avoid-biome sets unsupported'}
    (run/'result.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8',newline='\n')
    print(json.dumps(result,indent=2))


if __name__=='__main__':
    main()
