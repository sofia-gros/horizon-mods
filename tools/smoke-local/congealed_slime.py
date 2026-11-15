#!/usr/bin/env python3
"""Check production congealed collision boxes through the standard content loader.

Uses prepared Tinkers resources plus four byte-identical production block
bodies under a fixture namespace, with vanilla-texture models. Legacy shape
patches only target production IDs and cannot provide these fixture shapes.
This is a shape check, not bounce, fall immunity or piston verification.
"""
import argparse
import json
import os
import re
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
    parser.add_argument('--landing', action='store_true', help='check production player fall immunity/bounce and suppression against stone; not exact per-tick velocity parity')
    parser.add_argument('--mob-landing', action='store_true', help='check real cow fall immunity/bounce against nonfatal stone control; not all species or mob side contact')
    parser.add_argument('--item-landing', action='store_true', help='check dropped-item bounce and retained stack against stone and an unregistered inset control; not exact per-tick restitution or pistons')
    parser.add_argument('--entity-side', action='store_true', help='check falling cow/item side response and accepted world sound requests with an unregistered inset control; not all species, flat contact or audible/LAN parity')
    parser.add_argument('--side', action='store_true', help='check production side velocity reversal and suppression against stone; not sound/network parity')
    parser.add_argument('--side-drop', action='store_true', help='use a falling approach for side cases; does not replace the failed flat-approach gate')
    parser.add_argument('--side-face', choices=('west', 'east', 'north', 'south'), default='west', help='outward face exercised by side cases')
    parser.add_argument('--contact-sound', action='store_true', help='require accepted unit-volume/pitch native step-sound requests for ordinary falling-side contacts only; not audible playback')
    args = parser.parse_args()
    if args.contact_sound and not (args.side and args.side_drop):
        parser.error('--contact-sound requires --side --side-drop')
    game, controller, resources = (getattr(args, name).resolve() for name in ('game', 'ctl', 'resources'))
    pins = json.loads((ROOT / 'tconstruct/compatibility.json').read_text(encoding='utf-8'))
    require_report(game.parent, pins['horizon_revision'], pins['mixins_revision'], {'pluto_lib', 'pluto_probe', 'tconstruct'})
    run = Path(tempfile.mkdtemp(prefix='congealed-shapes-', dir=ROOT / '.local/smoke-runs')).resolve()
    content = run / 'content'
    content.mkdir()
    (content / 'horizon-mod.toml').write_text('id = "congealed_shape_probe"\nname = "Production congealed shapes"\nversion = "0.1.0"\napi = "0.2"\ncapabilities = ["registries"]\n', encoding='utf-8', newline='\n')
    for kind in ('earth', 'sky', 'ichor', 'ender'):
        name = kind + '_congealed_slime'
        path = Path(f'data/tconstruct/horizon/block/{name}.json')
        target = content / f'data/congealed_shape_probe/horizon/block/{name}.json'
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes((ROOT / 'tconstruct/content' / path).read_bytes())
        for relative, data in (
            (f'assets/congealed_shape_probe/blockstates/{name}.json', {'variants': {'': {'model': f'congealed_shape_probe:block/{name}'}}}),
            (f'assets/congealed_shape_probe/models/block/{name}.json', {'parent': 'minecraft:block/cube_all', 'textures': {'all': 'minecraft:block/slime_block'}}),
        ):
            target = content / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(json.dumps(data) + '\n', encoding='utf-8', newline='\n')
    env = dict(os.environ, HORIZON_NO_AUDIO='1', HORIZON_NO_DISCORD='1', HORIZON_TELEMETRY_URL='off', HORIZON_LAUNCHER_NO_SELF_UPDATE='1', RUST_MIN_STACK='16777216', PLUTO_PROBE_STRUCTURE_MODE='1')
    if args.contact_sound or args.entity_side or args.side:
        env['PLUTO_TRACE_INSIDE'] = '1'
    for key in ('PLUTO_PROBE_FEATURE_MODE', 'PLUTO_PROBE_NATURAL_MODE', 'PLUTO_PROBE_NATURAL_RESUME', 'PLUTO_PROBE_RESOURCE_PACK'):
        env.pop(key, None)
    for key in ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'TMP', 'TEMP'):
        folder = run / key.lower()
        folder.mkdir()
        env[key] = str(folder)
    endpoint = run / 'control.json'
    log = run / 'game.log'
    observations = []

    def ctl(*words, timeout=60):
        reply = subprocess.run([str(controller), '--socket', str(endpoint), '--timeout', str(timeout), *map(str, words)], capture_output=True, text=True, timeout=timeout+5)
        with (run / 'ctl.jsonl').open('a', encoding='utf-8', newline='\n') as output:
            output.write(json.dumps({'args': list(map(str, words)), 'code': reply.returncode, 'stdout': reply.stdout, 'stderr': reply.stderr})+'\n')
        if reply.returncode:
            raise RuntimeError(f'controller failed: {words}: {reply.stdout} {reply.stderr}; run={run}')
        value = json.loads(reply.stdout)
        if value.get('error') or value.get('ok') is False:
            raise RuntimeError(f'controller rejected {words}: {value}; run={run}')
        return value

    with log.open('w', encoding='utf-8', newline='\n') as output:
        proc = subprocess.Popen([str(game), '--resources', str(resources), '--dev-mod', str(content), '--control-socket', str(endpoint), '--offscreen', '--world', 'CongealedShapes', '--seed', '42', '--creative', '--test-terrain', '--render-distance', '2', '--width', '1280', '--height', '720'], cwd=run, env=env, stdout=output, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic()+300
            while not endpoint.exists():
                code = proc.poll()
                if code is not None:
                    raise RuntimeError(f'game exited before control socket: code={code}; run={run}')
                if time.monotonic()>deadline:
                    raise RuntimeError(f'game startup exceeded 300 seconds while still running; run={run}')
                time.sleep(.2)
            ctl('wait-ready', 120, timeout=125)
            ctl('command', 'time set 6000')
            for index, kind in enumerate(('earth', 'sky', 'ichor', 'ender')):
                x = index*8
                block = f'congealed_shape_probe:{kind}_congealed_slime'
                ctl('command', 'gamemode creative')
                ctl('command', f'fill {x-2} 99 -2 {x+2} 105 2 minecraft:air')
                ctl('setblock', x, 100, 0, block)
                ctl('command', f'tp @s {x+.5} 103 0.5')
                ctl('command', 'gamemode survival')
                ctl('wait', 100)
                state = ctl('state')
                position = state['player']['position']
                if abs(position[1]-100.9375)>0.001:
                    raise RuntimeError(f'{block}: wrong collision top: {position}; run={run}')
                ctl('command', 'gamemode spectator')
                ctl('command', f'tp @s {x+.5} 102 5.5')
                ctl('look-at', x+.5, 100.5, .5)
                ctl('key', 'f1', 'tap')
                image = run / f'{kind}.png'
                try:
                    ctl('screenshot', image)
                finally:
                    ctl('key', 'f1', 'tap')
                observations.append({'block': block, 'landed_position': position, 'screenshot': str(image)})
            if args.landing:
                # Production IDs exercise the registered consumer, unlike the
                # intentionally unregistered collision-shape aliases above.
                cases = [('minecraft:stone', False)] + [
                    (f'tconstruct:{kind}_congealed_slime', suppressed)
                    for kind in ('earth', 'sky', 'ichor', 'ender')
                    for suppressed in (False, True)
                ]
                for index, (block, suppressed) in enumerate(cases):
                    x = index * 8
                    ctl('key', 'shift', 'up')
                    ctl('command', 'gamemode creative')
                    ctl('command', f'tp @s {x+.5} 114 8.5')
                    ctl('command', f'fill {x-2} 99 6 {x+2} 116 10 minecraft:air')
                    ctl('command', f'fill {x-2} 100 6 {x+2} 100 10 {block}')
                    ctl('command', 'effect give @s minecraft:instant_health 1 10')
                    ctl('wait', 3)
                    before = ctl('state')['player']['health']
                    if suppressed:
                        ctl('key', 'shift', 'down')
                    ctl('command', 'gamemode survival')
                    samples = []
                    deadline = time.monotonic() + 12
                    while time.monotonic() < deadline:
                        player = ctl('state')['player']
                        samples.append(player)
                        if player['velocity'][1] > 0.3 or (player['on_ground'] and player['position'][1] < 102):
                            break
                        ctl('wait', 1)
                    # The first exposed grounded snapshot can precede the
                    # game's landing-event consumer. Retain it, then observe
                    # settled health without weakening the stone damage check.
                    if samples[-1]['on_ground'] and samples[-1]['velocity'][1] <= 0.3:
                        ctl('wait', 5)
                        samples.append(ctl('state')['player'])
                    rose = any(p['velocity'][1] > 0.3 for p in samples)
                    final = samples[-1]
                    if block == 'minecraft:stone':
                        valid = not rose and final['on_ground'] and final['health'] < before
                    else:
                        valid = final['health'] == before and (not rose and final['on_ground'] if suppressed else rose)
                    if not valid:
                        raise RuntimeError(f'landing failed: block={block} suppressed={suppressed} health_before={before} hurtlog={ctl("hurtlog")} samples={samples}; run={run}')
                    image = run / f'landing-{index}.png'
                    ctl('screenshot', image)
                    observations.append({'block': block, 'suppressed': suppressed, 'health_before': before, 'samples': samples, 'screenshot': str(image)})
                ctl('key', 'shift', 'up')
            if args.mob_landing:
                selector = '@e[type=minecraft:cow,tag=pluto_landing_probe,sort=nearest,limit=1]'
                def entity_number(path):
                    response = ctl('command', f'data get entity {selector} {path}')
                    text = '\n'.join(response.get('chat', []))
                    match = re.search(r':\s*(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)[dfbsL]?\s*$', text)
                    if not match:
                        raise RuntimeError(f'missing numeric cow {path}: {response}; run={run}')
                    return float(match.group(1))
                cases = ['minecraft:stone'] + [f'tconstruct:{kind}_congealed_slime' for kind in ('earth', 'sky', 'ichor', 'ender')]
                for index, block in enumerate(cases):
                    x = index*8
                    ctl('key', 'shift', 'up')
                    ctl('command', 'gamemode spectator')
                    ctl('command', 'kill @e[tag=pluto_landing_probe]')
                    ctl('wait', 25)
                    ctl('command', f'tp @s {x+4.5} 103 24.5')
                    ctl('command', f'fill {x-2} 99 22 {x+2} 116 26 minecraft:air')
                    ctl('command', f'fill {x-2} 100 22 {x+2} 100 26 {block}')
                    # This pin's NoAI suppresses travel as well as goals.
                    # A normal cow must actually fall; eight blocks leave the
                    # ten-health stone control alive for the post-fall read.
                    ctl('command', f'summon minecraft:cow {x+.5} 109 24.5 {{Tags:["pluto_landing_probe"]}}')
                    before = entity_number('Health')
                    samples = []
                    deadline = time.monotonic()+12
                    while time.monotonic() < deadline:
                        velocity = entity_number('Motion[1]')
                        height = entity_number('Pos[1]')
                        samples.append({'velocity_y': velocity, 'height': height})
                        if velocity > 0.3 or (height < 102 and abs(velocity) < 0.01):
                            break
                        ctl('wait', 1)
                    ctl('wait', 5)
                    health = entity_number('Health')
                    rose = any(row['velocity_y'] > 0.3 for row in samples)
                    valid = not rose and health < before if block == 'minecraft:stone' else rose and health == before
                    if not valid:
                        raise RuntimeError(f'mob landing failed: block={block} before={before} health={health} samples={samples}; run={run}')
                    height = entity_number('Pos[1]')
                    ctl('look-at', x+.5, height+.7, 24.5)
                    image = run / f'mob-landing-{index}.png'
                    ctl('screenshot', image)
                    observations.append({'block': block, 'mob': 'minecraft:cow', 'health_before': before, 'health_after': health, 'rose': rose, 'samples': samples, 'screenshot': str(image)})
                ctl('command', 'kill @e[tag=pluto_landing_probe]')
            if args.item_landing:
                selector = '@e[type=minecraft:item,tag=pluto_item_probe,sort=nearest,limit=1]'
                def item_number(path):
                    response = ctl('command', f'data get entity {selector} {path}')
                    text = '\n'.join(response.get('chat', []))
                    match = re.search(r':\s*(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)[dfbsL]?\s*$', text)
                    if not match:
                        raise RuntimeError(f'missing numeric item {path}: {response}; run={run}')
                    return float(match.group(1))
                cases = ['minecraft:stone', 'congealed_shape_probe:earth_congealed_slime'] + [f'tconstruct:{kind}_congealed_slime' for kind in ('earth', 'sky', 'ichor', 'ender')]
                for index, block in enumerate(cases):
                    x = index*8
                    ctl('command', 'gamemode spectator')
                    ctl('command', 'kill @e[tag=pluto_item_probe]')
                    ctl('command', f'tp @s {x+4.5} 103 32.5')
                    ctl('command', f'fill {x-2} 99 30 {x+2} 112 34 minecraft:air')
                    ctl('command', f'fill {x-2} 100 30 {x+2} 100 34 {block}')
                    ctl('command', f'summon minecraft:item {x+.5} 109 32.5 {{Tags:["pluto_item_probe"],PickupDelay:32767s,Item:{{id:"minecraft:cobblestone",count:1}}}}')
                    samples = []
                    deadline = time.monotonic()+10
                    while time.monotonic() < deadline:
                        velocity = item_number('Motion[1]')
                        height = item_number('Pos[1]')
                        samples.append({'velocity_y': velocity, 'height': height})
                        if velocity > 0.15 or (height < 102 and abs(velocity) < 0.05):
                            break
                        ctl('wait', 1)
                    rose = any(row['velocity_y'] > 0.15 for row in samples)
                    custom = block.startswith('tconstruct:')
                    landed = any(row['height'] < 102 for row in samples)
                    count = item_number('Item.count')
                    if rose != custom or (not custom and not landed) or count != 1:
                        raise RuntimeError(f'item landing failed: block={block} count={count} samples={samples}; run={run}')
                    height = item_number('Pos[1]')
                    ctl('look-at', x+.5, height+.2, 32.5)
                    image = run / f'item-landing-{index}.png'
                    ctl('screenshot', image)
                    observations.append({'block': block, 'entity': 'minecraft:item', 'count': count, 'rose': rose, 'samples': samples, 'screenshot': str(image)})
                ctl('command', 'kill @e[tag=pluto_item_probe]')
            if args.entity_side:
                normal_x, normal_z = {'west': (-1, 0), 'east': (1, 0), 'north': (0, -1), 'south': (0, 1)}[args.side_face]
                axis, sign = (0, normal_x) if normal_x else (2, normal_z)
                cases = ['congealed_shape_probe:earth_congealed_slime'] + [f'tconstruct:{kind}_congealed_slime' for kind in ('earth', 'sky', 'ichor', 'ender')]
                for family, offset in (('cow', 0.92), ('item', 0.60)):
                    selector = f'@e[type=minecraft:{family},tag=pluto_entity_side,sort=nearest,limit=1]'
                    def number(path):
                        response = ctl('command', f'data get entity {selector} {path}')
                        match = re.search(r':\s*(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)[dfbsL]?\s*$', '\n'.join(response.get('chat', [])))
                        if not match:
                            raise RuntimeError(f'missing {family} {path}: {response}; run={run}')
                        return float(match.group(1))
                    z_center = 40.5 if family == 'cow' else 48.5
                    for index, block in enumerate(cases):
                        x = index*8
                        ctl('command', 'gamemode spectator')
                        ctl('command', 'kill @e[tag=pluto_entity_side]')
                        ctl('command', f'tp @s {x+4.5} 103 {z_center+4}')
                        z = int(z_center)
                        ctl('command', f'fill {x-4} 100 {z-4} {x+4} 106 {z+4} minecraft:air')
                        ctl('command', f'fill {x-4} 99 {z-4} {x+4} 99 {z+4} minecraft:stone')
                        ctl('setblock', x, 100, z, block)
                        start_x, start_z = x+.5+offset*normal_x, z_center+offset*normal_z
                        nbt = '{Tags:["pluto_entity_side"],NoGravity:1b,Motion:[0d,0d,0d],'
                        nbt += 'NoAI:1b}' if family == 'cow' else 'PickupDelay:32767s,Item:{id:"minecraft:cobblestone",count:1}}'
                        ctl('command', f'summon minecraft:{family} {start_x} 103 {start_z} {nbt}')
                        initial = number(f'Pos[{axis}]')
                        expected = start_x if axis == 0 else start_z
                        if abs(initial-expected) > 0.05:
                            raise RuntimeError(f'entity-side staging moved: {family} initial={initial} expected={expected}; run={run}')
                        # NoAI stops this pin's travel; release it only after
                        # staging. A downward impulse makes the side response
                        # larger than ordinary cow wandering in the short fall.
                        ctl('command', f'data merge entity {selector} {{NoAI:0b,NoGravity:0b,Motion:[0d,-0.4d,0d]}}')
                        samples = []
                        floor_samples = 0
                        deadline = time.monotonic()+5
                        while time.monotonic() < deadline:
                            velocity = sign*number(f'Motion[{axis}]')
                            position = sign*(number(f'Pos[{axis}]')-initial)
                            height = number('Pos[1]')
                            samples.append({'outward_velocity': velocity, 'outward_displacement': position, 'height': height})
                            # Separate data-get replies are not an atomic
                            # snapshot: height can already be grounded while
                            # the earlier velocity read predates the contact.
                            floor_samples = floor_samples+1 if height < 100.1 else 0
                            if velocity > 0.25 or position > 0.15 or floor_samples >= 3:
                                break
                            ctl('wait', 1)
                        # Physical retreat is observable even when socket
                        # polling misses the peak. The post-run trace check
                        # independently requires >0.25 outward callback speed,
                        # synchronization and sound for that same entity.
                        reversed_motion = any(row['outward_velocity'] > 0.25 or row['outward_displacement'] > 0.15 for row in samples)
                        custom = block.startswith('tconstruct:')
                        if reversed_motion != custom or not samples or samples[-1]['height'] > 102:
                            raise RuntimeError(f'entity-side failed: {family} block={block} samples={samples}; run={run}')
                        count = number('Item.count') if family == 'item' else None
                        if family == 'item' and count != 1:
                            raise RuntimeError(f'entity-side consumed item: {count}; run={run}')
                        ctl('look-at', number('Pos[0]'), number('Pos[1]')+.4, number('Pos[2]'))
                        image = run / f'{family}-side-{index}.png'
                        ctl('screenshot', image)
                        observations.append({'block': block, 'entity_side': family, 'face': args.side_face, 'count': count, 'samples': samples, 'screenshot': str(image)})
                    ctl('command', 'kill @e[tag=pluto_entity_side]')
            if args.side:
                # Falling contact begins inside the unit block but outside the
                # inset shape. Use an unregistered identical inset shape as its
                # control; a full stone cube would start intersecting the body
                # and trigger native push-out rather than test the callback.
                control = 'congealed_shape_probe:earth_congealed_slime' if args.side_drop else 'minecraft:stone'
                cases = [(control, False)] + [
                    (f'tconstruct:{kind}_congealed_slime', suppressed)
                    for kind in ('earth', 'sky', 'ichor', 'ender')
                    for suppressed in (False, True)
                ]
                normal_x, normal_z = {'west': (-1, 0), 'east': (1, 0), 'north': (0, -1), 'south': (0, 1)}[args.side_face]
                for index, (block, suppressed) in enumerate(cases):
                    x = index * 8
                    def outward(player):
                        return (player['position'][0]-(x+.5))*normal_x + (player['position'][2]-16.5)*normal_z
                    ctl('key', 'w', 'up')
                    ctl('key', 'shift', 'up')
                    ctl('command', 'gamemode spectator')
                    ctl('command', f'tp @s {x-2.5} 102 16.5')
                    ctl('command', f'fill {x-4} 100 14 {x+3} 104 18 minecraft:air')
                    ctl('command', f'fill {x-4} 99 14 {x+3} 99 18 minecraft:stone')
                    ctl('setblock', x, 100, 16, block)
                    offset, start_y = (0.78, 103) if args.side_drop else (3.0, 100)
                    start_x, start_z = x+.5+offset*normal_x, 16.5+offset*normal_z
                    # Spectator staging avoids gravity/inside callbacks during
                    # socket commands. Start real physics only after all setup.
                    ctl('command', f'tp @s {start_x} {start_y} {start_z}')
                    ctl('look-at', x+.5-2*normal_x, start_y+1.62, 16.5-2*normal_z)
                    ctl('key', 'shift', 'down' if suppressed else 'up')
                    initial_side = ctl('state')['player']
                    # Shift descends in Spectator before Survival takes over.
                    # Require exact horizontal contact geometry and a bounded
                    # above-block height, not an impossible frozen Y coordinate.
                    px, py, pz = initial_side['position']
                    valid_height = start_y-0.5 <= py <= start_y+0.05 if args.side_drop else abs(py-start_y) <= 0.5
                    if abs(px-start_x) > 0.05 or abs(pz-start_z) > 0.05 or not valid_height:
                        raise RuntimeError(f'side staging moved outside prepared bounds: {initial_side}; run={run}')
                    ctl('command', 'gamemode survival')
                    if not args.side_drop:
                        ctl('wait', 5)
                    if suppressed:
                        ctl('key', 'shift', 'down')
                    samples = []
                    reversed_motion = False
                    reached = outward(initial_side) < 1.2
                    closest_outward = outward(initial_side)
                    if not args.side_drop:
                        ctl('key', 'w', 'down')
                    try:
                        deadline = time.monotonic() + 10
                        while time.monotonic() < deadline:
                            player = ctl('state')['player']
                            samples.append(player)
                            reached = reached or outward(player) < 1.2
                            # Socket polling can miss the short outward velocity
                            # interval. Retreat from the validated collision-free
                            # start is also observable; identical-shape/suppressed
                            # controls must show neither form of reversal.
                            current_outward = outward(player)
                            closest_outward = min(closest_outward, current_outward)
                            retreated = (current_outward > outward(initial_side)+0.05 if args.side_drop
                                else reached and current_outward > closest_outward+0.05)
                            outward_velocity = player['velocity'][0]*normal_x + player['velocity'][2]*normal_z
                            if reached and (outward_velocity > 0.01 or retreated):
                                reversed_motion = True
                                break
                            ctl('wait', 1)
                    finally:
                        ctl('key', 'w', 'up')
                        ctl('key', 'shift', 'up')
                    expect_reverse = block.startswith('tconstruct:') and not suppressed
                    if not reached or reversed_motion != expect_reverse:
                        raise RuntimeError(f'side failed: block={block} suppressed={suppressed} reached={reached} reversed={reversed_motion} samples={samples}; run={run}')
                    image = run / f'side-{index}.png'
                    ctl('screenshot', image)
                    observations.append({'block': block, 'side': True, 'face': args.side_face, 'falling_approach': args.side_drop, 'initial': initial_side, 'suppressed': suppressed, 'reversed': reversed_motion, 'samples': samples, 'screenshot': str(image)})
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
    if any(text.count('PLUTO-PROBE server_'+phase+' dedicated=false') != 1 for phase in ('starting','stopping','stopped')):
        raise RuntimeError(f'unclean lifecycle; run={run}')
    if args.contact_sound:
        sound_rows = re.findall(r'PLUTO-CONTACT-SOUND .*? sound=(\S+) at=(\[.*?\]) volume=(\S+) pitch=(\S+) accepted=(true|false)', text)
        expected = {(index*8+0.5, 100.5, 16.5) for index in (1,3,5,7)}
        seen = set()
        for sound, at, volume, pitch, accepted in sound_rows:
            pos = tuple(json.loads(at))
            if pos not in expected or sound != 'minecraft:block.slime_block.step' or float(volume) != 1.0 or float(pitch) != 1.0 or accepted != 'true':
                raise RuntimeError(f'wrong native contact sound: {(sound,at,volume,pitch,accepted)}; run={run}')
            seen.add(pos)
        if seen != expected:
            raise RuntimeError(f'missing native contact sounds: expected={expected} seen={seen}; run={run}')
    if args.entity_side:
        expected = {(index*8+0.5, 100.5, z) for index in range(1, 5) for z in (40.5, 48.5)}
        normal_x, normal_z = {'west': (-1, 0), 'east': (1, 0), 'north': (0, -1), 'south': (0, 1)}[args.side_face]
        axis, sign = (0, normal_x) if normal_x else (2, normal_z)
        eligible = set()
        contacts = re.findall(r'PLUTO-NONPLAYER-INSIDE entity=(\d+) kind=(\S+) block=(\S+) pos=(\[.*?\]) after=(\[.*?\]) marked=true sound=true', text)
        for entity, kind, block, at, velocity in contacts:
            pos = tuple(value+0.5 for value in json.loads(at))
            correct_kind = 'minecraft:cow' if pos[2] == 40.5 else 'minecraft:item'
            if pos in expected and kind == correct_kind and block.startswith('tconstruct:') and sign*json.loads(velocity)[axis] > 0.25:
                eligible.add((entity, pos))
        if {pos for _, pos in eligible} != expected:
            raise RuntimeError(f'missing entity-side callback peak/sync/sound: expected={expected} eligible={eligible}; run={run}')
        seen = set()
        rows = re.findall(r'PLUTO-WORLD-CONTACT-SOUND entity=(\d+) .*? sound=(\S+) at=(\[.*?\]) volume=(\S+) pitch=(\S+) accepted=(true|false)', text)
        for entity, sound, at, volume, pitch, accepted in rows:
            pos = tuple(json.loads(at))
            if pos[2] not in (40.5, 48.5):
                continue
            if pos not in expected or sound != 'minecraft:block.slime_block.step' or float(volume) != 1 or float(pitch) != 1 or accepted != 'true':
                raise RuntimeError(f'wrong entity-side sound: {(sound, at, volume, pitch, accepted)}; run={run}')
            if (entity, pos) in eligible:
                seen.add(pos)
        if seen != expected:
            raise RuntimeError(f'missing same-entity side sounds: expected={expected} seen={seen}; run={run}')
    scope = 'production collision shape through standard loader'
    if args.landing:
        scope += '; production player fall immunity, observed bounce/suppression and stone damage control; not exact per-tick velocity or pistons'
    elif not (args.mob_landing or args.item_landing):
        scope += '; bounce/fall immunity/pistons not tested'
    if args.mob_landing:
        scope += '; cow landing immunity and observed bounce with nonfatal stone damage control; other species and pistons not tested'
    if args.item_landing:
        scope += '; dropped-item observed bounce and retained stack with stone/unregistered-inset negatives; exact per-tick restitution and pistons not tested'
    if args.entity_side:
        scope += f'; falling cow/item side response and native world sound acceptance with inset controls, face={args.side_face}; flat contact, other species and audible/LAN parity unverified'
    if args.side:
        scope += '; side reversal/suppression with identical-inset control' if args.side_drop else '; held-forward flat side approach with stone control'
        scope += f'; face={args.side_face}; no separate packet-level or broader LAN check'
    if args.contact_sound:
        scope += '; native contact sound accepted at ordinary positions only, volume/pitch 1; audible playback unverified'
    result = {'result': 'CONGEALED MOTION PASS' if args.side or args.entity_side else ('CONGEALED LANDING PASS' if args.landing or args.mob_landing or args.item_landing else 'CONGEALED SHAPES PASS'), 'run': str(run), 'observations': observations, 'scope': scope}
    (run/'result.json').write_text(json.dumps(result, indent=2)+'\n', encoding='utf-8', newline='\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
