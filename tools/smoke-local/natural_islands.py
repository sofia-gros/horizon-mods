#!/usr/bin/env python3
"""Generate original End islands and optionally reopen their saved world.

Only the isolated clone's candidate set is made deterministic. Production
structure settings and biome tags stay unchanged. No /place commands, original
spacing/frequency parity, custom-fluid parity or LAN coverage.
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

BLOCKS = ('tconstruct:ender_slime_dirt', 'tconstruct:ichor_slime_dirt',
          'tconstruct:enderbark_log', 'tconstruct:ender_slime_leaves',
          'tconstruct:ender_slime_tall_grass', 'tconstruct:ender_slime_fern', 'tconstruct:ender_slime_vine')
CANDIDATES = ((2048,2048),(2560,2048),(2048,2560),(2560,2560),(3072,2048),(2048,3072))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('game', 'ctl', 'resources'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--islands', type=int, choices=range(1,7), default=1,
                        help='minimum fresh islands to observe before accepting tree/vine coverage')
    parser.add_argument('--resume', action='store_true', help='restart the same world and compare counts, sampled states and locate results')
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
    sets.pop('tconstruct:end_sky_island', None)  # no duplicate production start in the fixture
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
    phase = 'fresh'
    observations, screenshots = {}, []

    def ctl(*words, timeout=60):
        words = list(map(str, words))
        reply = subprocess.run([str(controller), '--socket', str(endpoint), '--timeout', str(timeout), *words],
                               capture_output=True, text=True, timeout=timeout+5)
        with (run / 'ctl.jsonl').open('a', encoding='utf-8', newline='\n') as output:
            output.write(json.dumps({'phase':phase,'args':words,'code':reply.returncode,'stdout':reply.stdout,'stderr':reply.stderr}) + '\n')
        if reply.returncode:
            raise RuntimeError(f'{phase}: control failed {words}: {reply.stdout} {reply.stderr}; run={run}')
        result = json.loads(reply.stdout)
        if result.get('error') or result.get('ok') is False:
            raise RuntimeError(f'{phase}: control rejected {words}: {result}; run={run}')
        return result

    def observe(index, candidate, saved=None):
        x,z = candidate
        ctl('command', f'execute in minecraft:the_end run tp @s {x+8} 160 {z+8}')
        ctl('wait',200)
        if saved is None:
            found = None
            # find-block caps radius at 64. Java SKY may be below y=96 over
            # End void columns, or near the generation-depth cap. Survey all
            # three overlapping vertical bands; do not label a low island absent.
            for altitude in (160,80,224):
                if altitude != 160:
                    ctl('command',f'tp @s {x+8} {altitude} {z+8}')
                    ctl('wait',80)
                ground = [ctl('find-block',name,64) for name in BLOCKS[:2]]
                found = next((row for row in ground if row.get('count',0)>0),None)
                if found is not None:
                    break
            if found is None:
                raise RuntimeError(f'natural original island absent across End height bands at {candidate}; run={run}')
            focus = found['nearest'][0]['pos']
        else:
            focus = saved['focus']
        # Count from the same settled camera/loaded-region footprint in both
        # processes; a changed player position must not change the census area.
        ctl('command',f'tp @s {focus[0]+28} {focus[1]+18} {focus[2]+28}')
        ctl('look-at',focus[0],focus[1]+3,focus[2])
        ctl('wait',200)
        found = {name:ctl('find-block',name,64) for name in BLOCKS}
        counts = {name:value.get('count',0) for name,value in found.items()}
        if sum(counts[name] for name in BLOCKS[:2])<=0:
            raise RuntimeError(f'{phase}: natural island ground disappeared; run={run}')
        samples = []
        if saved is None:
            for name in BLOCKS:
                for item in found[name].get('nearest',[])[:3]:
                    position = item['pos']
                    samples.append({'pos':position,'state':ctl('getblock',*position)['block']})
        else:
            for item in saved['samples']:
                samples.append({'pos':item['pos'],'state':ctl('getblock',*item['pos'])['block']})
        located = ctl('command','locate structure tconstruct:end_slime_island')
        if not any('nearest' in text.lower() and 'tconstruct:end_slime_island' in text for text in located.get('chat',[])):
            raise RuntimeError(f'{phase}: generated start not locatable: {located}; run={run}')
        snapshot = {'candidate':list(candidate),'focus':focus,'counts':counts,'samples':samples,'locate':located}
        if saved is not None and snapshot != saved:
            raise RuntimeError(f'saved island changed: expected={saved}, actual={snapshot}; run={run}')
        ctl('key','f1','tap')
        try:
            image = run / f'{phase}-island-{index}.png'
            ctl('screenshot',image)
            if not image.is_file() or not image.stat().st_size:
                raise RuntimeError('missing natural island screenshot')
            screenshots.append(str(image))
        finally:
            ctl('key','f1','tap')
        return snapshot

    for phase in (('fresh','resume') if args.resume else ('fresh',)):
        endpoint.unlink(missing_ok=True)  # the preceding process exited cleanly
        log = run / (phase+'.log')
        with log.open('w',encoding='utf-8',newline='\n') as output:
            proc = subprocess.Popen([str(game),'--resources',str(resources),'--control-socket',str(endpoint),
                '--offscreen','--world','NaturalIslands','--seed','42','--creative',
                '--render-distance','4','--width','1280','--height','720'],cwd=run,env=env,
                stdout=output,stderr=subprocess.STDOUT)
            try:
                deadline = time.monotonic()+120
                while not endpoint.exists():
                    if proc.poll() is not None or time.monotonic()>deadline:
                        raise RuntimeError(f'{phase}: game did not start; run={run}')
                    time.sleep(.2)
                ctl('wait-ready',120,timeout=125)
                for command in ('gamemode spectator','gamerule random_tick_speed 0','time set 6000','weather clear'):
                    ctl('command',command)
                rows = []
                if phase=='fresh':
                    for index,candidate in enumerate(CANDIDATES):
                        rows.append(observe(index,candidate))
                        if (len(rows)>=args.islands
                            and any(row['counts']['tconstruct:enderbark_log']>0 and row['counts']['tconstruct:ender_slime_leaves']>0 for row in rows)
                            and any(sum(row['counts'][name] for name in BLOCKS[4:])>0 for row in rows)):
                            break
                    else:
                        raise RuntimeError(f'insufficient original tree and grass/vine markers across six natural islands; run={run}')
                else:
                    for index,saved in enumerate(observations['fresh']):
                        rows.append(observe(index,saved['candidate'],saved))
                observations[phase] = rows
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
        # Retain successful observations even when a later persistence/lifecycle
        # assertion fails; diagnostic evidence is not a passing result receipt.
        (run/'observations.json').write_text(json.dumps(observations,indent=2)+'\n',encoding='utf-8',newline='\n')
        text = log.read_text(encoding='utf-8',errors='replace')
        for status in ('starting','stopping','stopped'):
            if text.count(f'PLUTO-PROBE server_{status} dedicated=false')!=1:
                raise RuntimeError(f'{phase}: unclean lifecycle {status}; run={run}')
        for failure in ('registered piece requires the typed placement adapter','registered structure state invalid',
                        'natural structure tconstruct:end_slime_island rejected','tree configuration failed','tree placement failed'):
            if failure in text:
                raise RuntimeError(f'{phase}: natural marker failure {failure}; run={run}')
        if phase=='resume' and 'pluto_lib: restored saved structure piece kind=pluto_lib:registered_template_piece' not in text:
            raise RuntimeError(f'resume did not observe registered island piece restoration; run={run}')
    result = {'result':'NATURAL ISLANDS PASS','run':str(run),'resume_checked':args.resume,
              'observations':observations,'screenshots':screenshots,
              'scope':'original settings on deterministic fixture candidates; distribution/custom-fluid/LAN parity not checked'}
    (run/'result.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8',newline='\n')
    print(json.dumps(result,indent=2))


if __name__=='__main__':
    main()
