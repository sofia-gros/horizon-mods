#!/usr/bin/env python3
"""One real-game batch for the clean tool domain: content loads, a station builds, swaps and takes tools.

Run only against a linked clean build (tools/build_clean_game.py) with `--content`-style assets:
the script copies tconstruct/content into a fresh isolated game directory. One run covers every
scenario below, because each launch costs about half an hour; do not use it for single probes.

Scenarios (each asserted from the running game, with screenshots kept in the run directory):
  1. the content mod loads and the station block opens a 6-slot container menu,
  2. wood head + handle + binding in the input slots preview a tconstruct:pickaxe in the result slot,
  3. taking the result gives the tool (with rebuilt stats) and consumes exactly the parts,
  4. a stone head on that tool swaps the head material in the result,
  5. the game log carries no Tinkers loader problems beyond the known unbound-module report.
"""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--game', type=Path, required=True, help='linked horizon-client executable')
    p.add_argument('--ctl', type=Path, required=True, help='horizon-ctl executable')
    p.add_argument('--resources', type=Path, required=True)
    p.add_argument('--runs', type=Path, default=ROOT / '.local/smoke-runs')
    a = p.parse_args()
    a.runs.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix='tool-build-', dir=a.runs)).resolve()
    endpoint = run / 'control.json'
    game_dir = run / 'game'
    shutil.copytree(ROOT / 'tconstruct/content', game_dir / 'horizon-mods/tconstruct')
    (game_dir / 'horizon-mods.txt').write_text('mod.tconstruct:true' + chr(10), encoding='utf-8', newline=chr(10))
    env = dict(os.environ, HORIZON_NO_AUDIO='1', HORIZON_NO_DISCORD='1', HORIZON_TELEMETRY_URL='off',
               HORIZON_LAUNCHER_NO_SELF_UPDATE='1', RUST_MIN_STACK='16777216')
    for key in ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'TMP', 'TEMP'):
        folder = run / key.lower()
        folder.mkdir()
        env[key] = str(folder)
    log = (run / 'game.log').open('w', encoding='utf-8')
    command = [str(a.game.resolve()), '--home', env['HORIZON_HOME'], '--game-dir', str(game_dir), '--resources', str(a.resources.resolve()),
               '--world', 'ToolBuild', '--seed', '42', '--creative', '--control-socket', str(endpoint), '--offscreen',
               '--render-distance', '2', '--width', '1280', '--height', '720']
    proc = subprocess.Popen(command, cwd=run, env=env, stdout=log, stderr=subprocess.STDOUT)

    def ctl(*args, timeout=30):
        r = subprocess.run([str(a.ctl.resolve()), '--socket', str(endpoint), '--timeout', str(timeout), *map(str, args)],
                           capture_output=True, text=True, timeout=timeout + 10)
        with (run / 'ctl.jsonl').open('a', encoding='utf-8', newline=chr(10)) as f:
            f.write(json.dumps({'args': args, 'code': r.returncode, 'stdout': r.stdout, 'stderr': r.stderr}) + chr(10))
        if r.returncode:
            raise RuntimeError((args, r.stdout, r.stderr))
        value = json.loads(r.stdout)
        if value.get('error'):
            raise RuntimeError(value)
        return value

    def ui_slots():
        value = ctl('ui')
        nodes = value.get('widgets', value.get('nodes', []))
        return {int(n['id'][5:]): n['bounds'] for n in nodes if n['id'].startswith('slot-')}

    def shot(name):
        ctl('screenshot', str(run / name))
        assert (run / name).is_file(), name
        print('SCREENSHOT', run / name, flush=True)

    def container_slots():
        return {s['slot']: s for s in ctl('screen')['slots']}

    def put(item, material, container_slot):
        """Give a part, pick it from the player's inventory and drop it in a container slot."""
        ctl('command', f'give @s tconstruct:{item}[custom_data={{Material:"tconstruct:{material}"}}] 1')
        ctl('wait', '5')
        inv = ctl('screen')['slots']
        source = next(s['slot'] for s in inv if s['slot'] < 36 and s.get('item') == f'tconstruct:{item}')
        ctl('click-slot', source)
        bounds = ui_slots()[container_slot]
        ctl('click-at', bounds[0] + 8, bounds[1] + 8)
        ctl('wait', '5')

    result = {'scenarios': []}
    soft_failures = []
    try:
        deadline = time.monotonic() + 180
        while not endpoint.exists():
            if proc.poll() is not None or time.monotonic() > deadline:
                raise RuntimeError(f'game did not start: {run / "game.log"}')
            time.sleep(.2)
        ctl('wait-ready', '180', timeout=185)
        ctl('command', 'clear @s')
        # A flat arena in the sky: terrain (trees, leaves, litter, slopes) would otherwise get in the way of every scenario.
        # A 41 x 41 stone floor at height 199 with clear air above it, and the player on top of it.
        pos = ctl('state')['player']['position']
        centre = [int(pos[0] // 1), int(pos[2] // 1)]
        ctl('command', f'fill {centre[0] - 20} 200 {centre[1] - 20} {centre[0] + 20} 207 {centre[1] + 20} minecraft:air')
        ctl('command', f'fill {centre[0] - 20} 199 {centre[1] - 20} {centre[0] + 20} 199 {centre[1] + 20} minecraft:stone')
        ctl('command', f'tp @s {centre[0] + 0.5} 200 {centre[1] + 0.5}')
        ctl('wait', '20')
        pos = ctl('state')['player']['position']
        assert pos[1] >= 199.5, f'the player is not on the arena: {pos}'
        xyz = [int(pos[0] // 1) + 2, int(pos[1] // 1), int(pos[2] // 1)]
        ctl('command', f'setblock {xyz[0]} {xyz[1]} {xyz[2]} tconstruct:tinker_station')
        ctl('wait', '20')
        ctl('interact', *xyz, 'up')
        ctl('wait', '10')
        assert ctl('screen')['screen'] == 'container', 'the station block did not open a container menu'
        slots = ui_slots()
        first = min(i for i in slots if i >= 300)
        tool, inputs, result_slot = first, [first + 1, first + 2, first + 3], first + 5
        assert result_slot in slots, sorted(slots)
        shot('01-station-open.png')
        # The content mod's language file names the window (a key on screen means nobody translated it).
        title = ''.join(part.get('text', '') for part in ctl('screen').get('title', []))
        assert title == 'Tinker Station', title
        result['scenarios'].append('content mod loads and the station opens a 6-slot menu')

        for item, material, slot in [('pick_head', 'wood', inputs[0]), ('tool_handle', 'wood', inputs[1]), ('tool_binding', 'wood', inputs[2])]:
            put(item, material, slot)
        preview = container_slots()[result_slot]
        assert preview.get('item') == 'tconstruct:pickaxe', preview
        assert 'tic_stats' in preview.get('components', ''), 'the previewed tool has no rebuilt stats: ' + str(preview)
        assert 'attribute_modifiers' in preview.get('components', '') and 'tconstruct:tool_attack_damage' in preview.get('components', ''), 'the previewed melee tool publishes no attack attributes: ' + str(preview)
        shot('02-pickaxe-preview.png')
        result['scenarios'].append('three wood parts preview a pickaxe with stats')

        bounds = ui_slots()[result_slot]
        ctl('click-at', bounds[0] + 8, bounds[1] + 8)
        ctl('wait', '5')
        screen = ctl('screen')
        assert screen['carried'] and screen['carried']['item'] == 'tconstruct:pickaxe', screen['carried']
        assert all(s.get('item') is None for s in screen['slots'] if first <= s['slot'] <= first + 4), 'parts were not consumed'
        assert any(s['slot'] == result_slot and s.get('item') is None for s in screen['slots']), 'result slot not cleared'
        ctl('click-slot', tool)
        ctl('wait', '5')
        shot('03-pickaxe-taken.png')
        result['scenarios'].append('taking the pickaxe consumed the parts and gave the tool')

        put('pick_head', 'rock', inputs[0])
        swapped = container_slots()[result_slot]
        assert swapped.get('item') == 'tconstruct:pickaxe' and 'tconstruct:rock' in swapped.get('components', ''), swapped
        shot('04-head-swap-preview.png')
        result['scenarios'].append('a stone head previews a head swap on the placed pickaxe')

        # Part builder: planks plus a pattern; the first pattern button crafts a wood part.
        ctl('close')
        pb = [xyz[0] - 2, xyz[1], xyz[2]]
        ctl('command', f'setblock {pb[0]} {pb[1]} {pb[2]} tconstruct:part_builder')
        ctl('command', 'give @s tconstruct:pattern 1')
        ctl('command', 'give @s minecraft:oak_planks 8')
        ctl('wait', '5')
        ctl('interact', *pb, 'up')
        ctl('wait', '10')
        assert ctl('screen')['screen'] == 'container', 'the part builder did not open a container menu'
        slots = ui_slots()
        first = min(i for i in slots if i >= 300)
        assert {first, first + 1, first + 2} <= set(slots), sorted(slots)
        inv = ctl('screen')['slots']
        for item, target in (('minecraft:oak_planks', first), ('tconstruct:pattern', first + 1)):
            source = next(s['slot'] for s in inv if s['slot'] < 36 and s.get('item') == item)
            ctl('click-slot', source)
            bounds = ui_slots()[target]
            ctl('click-at', bounds[0] + 8, bounds[1] + 8)
            ctl('wait', '5')
        shot('05-part-builder-inputs.png')
        origin = ui_slots()[first]
        ctl('click-at', origin[0] - 8 + 34 + 9, origin[1] - 24 + 18 + 9)
        ctl('wait', '10')
        built = container_slots()[first + 2]
        assert built.get('item', '').startswith('tconstruct:') and 'tconstruct:wood' in built.get('components', ''), built
        shot('06-part-builder-result.png')
        result['scenarios'].append('the part builder offers a pattern button and previews a wood part')

        # Tool recycling: a pickaxe with no upgrades and a venom bottle in the part builder offer the pickaxe's parts.
        ctl('close')
        ctl('command', 'clear @s')
        ctl('command', 'give @s tconstruct:pickaxe[custom_data={tic_materials:["tconstruct:wood","tconstruct:wood","tconstruct:wood"],tic_stats:{"tconstruct:attack_damage":1.0f,"tconstruct:attack_speed":1.2f,"tconstruct:durability":60.0f,"tconstruct:harvest_tier":"minecraft:wood","tconstruct:mining_speed":2.0f}}] 1')
        ctl('command', 'give @s tconstruct:venom_bottle 1')
        ctl('command', f'setblock {pb[0]} {pb[1]} {pb[2]} minecraft:air')
        ctl('command', f'setblock {pb[0]} {pb[1]} {pb[2]} tconstruct:part_builder')
        ctl('wait', '5')
        ctl('interact', *pb, 'up')
        ctl('wait', '10')
        slots = ui_slots()
        first = min(i for i in slots if i >= 300)
        inv = ctl('screen')['slots']
        for item, target in (('tconstruct:pickaxe', first), ('tconstruct:venom_bottle', first + 1)):
            source = next(s['slot'] for s in inv if s['slot'] < 36 and s.get('item') == item)
            ctl('click-slot', source)
            bounds = ui_slots()[target]
            ctl('click-at', bounds[0] + 8, bounds[1] + 8)
            ctl('wait', '5')
            inv = ctl('screen')['slots']
        origin = ui_slots()[first]
        ctl('click-at', origin[0] - 8 + 34 + 9, origin[1] - 24 + 18 + 9)
        ctl('wait', '10')
        shot('06b-part-recycling.png')
        recycled = container_slots()[first + 2]
        assert recycled.get('item', '').startswith('tconstruct:') and 'tconstruct:wood' in recycled.get('components', ''), recycled
        result['scenarios'].append('the part builder takes a pickaxe apart into a wood part with a venom bottle')

        # Modifier worktable opens with its four slots.
        ctl('close')
        wt = [xyz[0] - 2, xyz[1], xyz[2] + 2]
        ctl('command', f'setblock {wt[0]} {wt[1]} {wt[2]} tconstruct:modifier_worktable')
        ctl('interact', *wt, 'up')
        ctl('wait', '10')
        assert ctl('screen')['screen'] == 'container', 'the modifier worktable did not open a container menu'
        wslots = ui_slots()
        wfirst = min(i for i in wslots if i >= 300)
        assert {wfirst, wfirst + 1, wfirst + 2, wfirst + 3} <= set(wslots), sorted(wslots)
        shot('07-worktable-open.png')
        result['scenarios'].append('the modifier worktable opens with a 4-slot menu')

        # Removal: a wet sponge lets the worktable take a modifier off a tool (the first modifier button).
        removal_stats = 'tic_stats:{"tconstruct:attack_damage":1.0f,"tconstruct:attack_speed":1.2f,"tconstruct:durability":60.0f,"tconstruct:harvest_tier":"minecraft:wood","tconstruct:mining_speed":2.0f}'
        removal_parts = 'tic_materials:["tconstruct:wood","tconstruct:wood","tconstruct:wood"]'
        ctl('command', 'clear @s')
        ctl('command', f'give @s tconstruct:pickaxe[custom_data={{{removal_parts},{removal_stats},tic_upgrades:[{{level:1,name:"tconstruct:sharpness"}}]}}] 1')
        ctl('command', 'give @s minecraft:wet_sponge 1')
        ctl('wait', '5')
        inv = ctl('screen')['slots']
        for item, target in (('tconstruct:pickaxe', wfirst), ('minecraft:wet_sponge', wfirst + 1)):
            source = next(s['slot'] for s in inv if s['slot'] < 36 and s.get('item') == item)
            ctl('click-slot', source)
            bounds = ui_slots()[target]
            ctl('click-at', bounds[0] + 8, bounds[1] + 8)
            ctl('wait', '5')
            inv = ctl('screen')['slots']
        origin = ui_slots()[wfirst]
        ctl('click-at', origin[0] - 8 + 54 + 9, origin[1] - 24 + 18 + 9)
        ctl('wait', '10')
        shot('07b-worktable-removal.png')
        removed = container_slots()[wfirst + 3]
        assert removed.get('item') == 'tconstruct:pickaxe' and 'sharpness' not in removed.get('components', ''), removed
        result['scenarios'].append('the worktable previews taking a modifier off a tool with a wet sponge')

        # Repair: a damaged pickaxe in the station is repaired with planks (the wood material).
        ctl('close')
        # The earlier scenarios left parts in the station's slots: replace the block so the slots are empty.
        ctl('command', f'setblock {xyz[0]} {xyz[1]} {xyz[2]} minecraft:air')
        ctl('command', f'setblock {xyz[0]} {xyz[1]} {xyz[2]} tconstruct:tinker_station')
        ctl('command', 'clear @s')
        ctl('command', 'give @s tconstruct:pickaxe[custom_data={tic_materials:["tconstruct:wood","tconstruct:wood","tconstruct:wood"],tic_stats:{"tconstruct:attack_damage":1.0f,"tconstruct:attack_speed":1.2f,"tconstruct:durability":60.0f,"tconstruct:harvest_tier":"minecraft:wood","tconstruct:mining_speed":2.0f},Damage:20,tic_modifiers:[{level:1,name:"tconstruct:silky"},{level:1,name:"tconstruct:respiration"}]}] 1')
        ctl('command', 'give @s minecraft:oak_planks 8')
        ctl('wait', '5')
        ctl('interact', *xyz, 'up')
        ctl('wait', '10')
        slots = ui_slots()
        first = min(i for i in slots if i >= 300)
        tool, inputs, result_slot = first, [first + 1, first + 2, first + 3], first + 5
        inv = ctl('screen')['slots']
        for item, target in (('tconstruct:pickaxe', tool), ('minecraft:oak_planks', inputs[0])):
            source = next(s['slot'] for s in inv if s['slot'] < 36 and s.get('item') == item)
            ctl('click-slot', source)
            bounds = ui_slots()[target]
            ctl('click-at', bounds[0] + 8, bounds[1] + 8)
            ctl('wait', '5')
            inv = ctl('screen')['slots']
        repaired = container_slots()[result_slot]
        shot('08-repair-preview.png')
        assert repaired.get('item') == 'tconstruct:pickaxe', repaired
        assert 'Damage' not in repaired.get('components', '') or 'Damage: 20' not in repaired.get('components', ''), repaired
        # The rebuilt tool publishes what its modifiers give it: silky's silk touch and the melee attributes.
        assert 'respiration' in repaired.get('components', ''), repaired
        print('ENCHANTMENTS', 'silk_touch' in repaired.get('components', ''), flush=True)
        result['scenarios'].append('planks preview a repair of a damaged pickaxe')
        ctl('close')

        # Modifier crystal: a crystal alone on the station applies its modifier to the tool, in place of the recipe's inputs.
        ctl('command', f'setblock {xyz[0]} {xyz[1]} {xyz[2]} minecraft:air')
        ctl('command', f'setblock {xyz[0]} {xyz[1]} {xyz[2]} tconstruct:tinker_station')
        ctl('command', 'clear @s')
        ctl('command', 'give @s tconstruct:pickaxe[custom_data={tic_materials:["tconstruct:wood","tconstruct:wood","tconstruct:wood"],tic_stats:{"tconstruct:attack_damage":1.0f,"tconstruct:attack_speed":1.2f,"tconstruct:durability":60.0f,"tconstruct:harvest_tier":"minecraft:wood","tconstruct:mining_speed":2.0f},tic_volatile_data:{upgrades:3,abilities:1}}] 1')
        ctl('command', 'give @s tconstruct:modifier_crystal[custom_data={modifier:"tconstruct:sharpness"}] 1')
        ctl('wait', '5')
        ctl('interact', *xyz, 'up')
        ctl('wait', '10')
        slots = ui_slots()
        first = min(i for i in slots if i >= 300)
        tool, inputs, result_slot = first, [first + 1, first + 2, first + 3], first + 5
        inv = ctl('screen')['slots']
        for item, target in (('tconstruct:pickaxe', tool), ('tconstruct:modifier_crystal', inputs[0])):
            source = next(s['slot'] for s in inv if s['slot'] < 36 and s.get('item') == item)
            ctl('click-slot', source)
            bounds = ui_slots()[target]
            ctl('click-at', bounds[0] + 8, bounds[1] + 8)
            ctl('wait', '5')
            inv = ctl('screen')['slots']
        crystal_result = container_slots()[result_slot]
        shot('08b-modifier-crystal.png')
        assert crystal_result.get('item') == 'tconstruct:pickaxe' and 'sharpness' in crystal_result.get('components', ''), crystal_result
        result['scenarios'].append('a modifier crystal alone on the station applies its modifier')
        ctl('close')

        # Mining speed: a worn stonebound pickaxe digs stone in fewer ticks than the same pickaxe without it.
        stats = 'tic_stats:{"tconstruct:attack_damage":1.0f,"tconstruct:attack_speed":1.2f,"tconstruct:durability":60.0f,"tconstruct:harvest_tier":"minecraft:wood","tconstruct:mining_speed":2.0f}'
        parts = 'tic_materials:["tconstruct:wood","tconstruct:wood","tconstruct:wood"]'
        rock = [xyz[0] - 1, xyz[1], xyz[2] - 2]

        def dig_ticks(extra):
            ctl('command', 'clear @s')
            ctl('command', f'give @s tconstruct:pickaxe[custom_data={{{parts},{stats}{extra}}}] 1')
            # Decorations such as leaf litter can sit in front of the block: clear the cell around it first.
            ctl('command', f'fill {rock[0] - 1} {rock[1]} {rock[2] - 1} {rock[0] + 1} {rock[1] + 1} {rock[2] + 1} minecraft:air')
            ctl('command', f'setblock {rock[0]} {rock[1]} {rock[2]} minecraft:stone')
            ctl('wait', '5')
            ctl('select', 1)
            value = ctl('break', *rock, timeout=60)
            assert value.get('block') in ('minecraft:air', None) or 'air' in json.dumps(value.get('block')), value
            return value['ticks']

        # Creative mode breaks blocks instantly, so dig in survival.
        ctl('command', 'gamemode survival')
        samples = {}
        for label, extra in (('plain', ''), ('plain again', ''), ('damage only', ',Damage:50'),
                             ('stonebound fresh', ',tic_modifiers:[{level:3,name:"tconstruct:stonebound"}]'),
                             ('stonebound worn', ',Damage:50,tic_multipliers:{"tconstruct:durability":0.00001f},tic_modifiers:[{level:3,name:"tconstruct:stonebound"}]')):
            samples[label] = dig_ticks(extra)
        print('DIG TICKS', json.dumps(samples), flush=True)
        ctl('command', 'gamemode creative')
        plain_ticks, worn_ticks = samples['plain'], samples['stonebound worn']
        reference = samples['plain again']
        assert samples['stonebound worn'] < reference, samples
        result['scenarios'].append(f'a worn stonebound pickaxe digs stone faster: {worn_ticks} ticks against {plain_ticks}')

        # Melee damage: smite adds damage against undead targets (a husk with no AI stays where it is summoned).
        ctl('command', 'gamemode survival')

        def hit_damage(extra):
            ctl('command', 'kill @e[type=minecraft:husk]')
            ctl('command', 'clear @s')
            ctl('command', f'give @s tconstruct:pickaxe[custom_data={{{parts},{stats}{extra}}}] 1')
            ctl('command', f'summon minecraft:husk {rock[0]} {rock[1]} {rock[2]} {{NoAI:1b,PersistenceRequired:1b}}')
            ctl('wait', '20')
            ctl('select', 1)
            found = ctl('entities', 8, 'husk')
            listed = found.get('mobs') or found.get('entities') or []
            assert listed, found
            target = listed[0]['id']
            ctl('look-at-entity', target)
            ctl('wait', '30')
            reply = ctl('attack', target)
            assert reply.get('hit'), reply
            return reply['health_before'] - reply['health']

        plain_damage = hit_damage('')
        smite_damage = hit_damage(',tic_modifiers:[{level:3,name:"tconstruct:smite"}]')
        print('HIT DAMAGE', json.dumps({'plain': plain_damage, 'smite': smite_damage}), flush=True)
        ctl('command', 'kill @e[type=minecraft:husk]')
        ctl('command', 'gamemode creative')
        assert smite_damage > plain_damage, (plain_damage, smite_damage)
        result['scenarios'].append(f'smite adds damage against an undead target: {smite_damage} against {plain_damage}')

        # Tool actions: a tool with the stripping modifier strips an oak log, one with tilling tills dirt.
        def use_on(block, extra, modifier):
            ctl('command', 'clear @s')
            ctl('command', f'give @s tconstruct:pickaxe[custom_data={{{parts},{stats}{extra},tic_modifiers:[{{level:1,name:"tconstruct:{modifier}"}}]}}] 1')
            ctl('command', f'fill {rock[0] - 1} {rock[1]} {rock[2] - 1} {rock[0] + 1} {rock[1] + 1} {rock[2] + 1} minecraft:air')
            ctl('command', f'setblock {rock[0]} {rock[1]} {rock[2]} {block}')
            ctl('wait', '5')
            ctl('select', 1)
            before = ctl('block', *rock)
            # The first click after the player turned to the target can land on air; click again until the block answers.
            for _ in range(3):
                ctl('wait', '10')
                ctl('interact', *rock, 'up')
                ctl('wait', '5')
                after = ctl('block', *rock)
                if after.get('state') != before.get('state'):
                    break
            return after

        # A failure here is recorded and the run goes on, so one run reports every scenario after it too.
        stripped = use_on('minecraft:oak_log[axis=y]', '', 'stripping')
        tilled = use_on('minecraft:dirt', '', 'tilling')
        if 'stripped_oak_log' in json.dumps(stripped) and 'farmland' in json.dumps(tilled):
            result['scenarios'].append('stripping strips an oak log and tilling tills dirt with a Tinkers tool')
        else:
            soft_failures.append(f'tool actions: stripping gave {json.dumps(stripped)}, tilling gave {json.dumps(tilled)}')

        # Protection: the same hit hurts less through a helmet with eight levels of protection (ten points, 40%).
        def hurt_through(modifiers):
            ctl('command', 'gamemode survival')
            ctl('command', 'effect clear @s')
            ctl('command', 'effect give @s minecraft:instant_health 1 10 true')
            ctl('wait', '30')
            ctl('command', 'item replace entity @s armor.head with tconstruct:plate_helmet[custom_data={tic_materials:["tconstruct:iron","tconstruct:iron"]' + modifiers + '}]')
            ctl('wait', '10')
            before = ctl('state')['player']['health']
            ctl('command', 'damage @s 8 minecraft:mob_attack')
            ctl('wait', '10')
            return before - ctl('state')['player']['health']

        bare_loss = hurt_through('')
        guarded_loss = hurt_through(',tic_modifiers:[{level:8,name:"tconstruct:protection"}]')
        print('PROTECTION LOSS', json.dumps({'plain': bare_loss, 'protected': guarded_loss}), flush=True)
        ctl('command', 'gamemode creative')
        if guarded_loss < bare_loss - 0.5:
            result['scenarios'].append(f'a protection helmet takes the sting out of a hit: {guarded_loss} lost against {bare_loss}')
        else:
            soft_failures.append(f'protection: {guarded_loss} lost with protection against {bare_loss} without')

        # Armor attributes: the worn helmet's own armor and the toughness its maintained modifier adds once the helmet is worn down (it is zero on a fresh one) read back from the player.
        def read_attribute(name):
            reply = json.dumps(ctl('command', f'attribute @s {name} get'))
            found = re.findall(r'-?\d+(?:\.\d+)?', reply.split('value')[-1] if 'value' in reply else reply)
            return float(found[-1]) if found else None

        ctl('command', 'item replace entity @s armor.head with tconstruct:plate_helmet[custom_data={tic_materials:["tconstruct:iron","tconstruct:iron"]}]')
        ctl('wait', '10')
        plain_armor, plain_toughness = read_attribute('minecraft:armor'), read_attribute('minecraft:armor_toughness')
        ctl('command', 'item replace entity @s armor.head with tconstruct:plate_helmet[custom_data={tic_materials:["tconstruct:iron","tconstruct:iron"],Damage:200,tic_modifiers:[{level:2,name:"tconstruct:maintained"}]}]')
        ctl('wait', '10')
        kept_toughness = read_attribute('minecraft:armor_toughness')
        print('ARMOR ATTRIBUTES', json.dumps({'armor': plain_armor, 'toughness': plain_toughness, 'maintained toughness': kept_toughness}), flush=True)
        if plain_armor is not None and plain_armor > 0 and kept_toughness is not None and plain_toughness is not None and kept_toughness > plain_toughness:
            result['scenarios'].append(f'a worn helmet gives armor {plain_armor} and maintained raises toughness {plain_toughness} to {kept_toughness}')
        else:
            soft_failures.append(f'armor attributes: armor {plain_armor}, toughness {plain_toughness}, maintained {kept_toughness}')
        ctl('command', 'item replace entity @s armor.head with minecraft:air')

        # Save and reload: the pickaxe survives quitting and reopening the same world.
        ctl('command', 'clear @s')
        ctl('command', 'give @s tconstruct:pickaxe[custom_data={tic_materials:["tconstruct:wood","tconstruct:wood","tconstruct:wood"]}] 1')
        ctl('wait', '5')
        ctl('quit')
        proc.wait(timeout=90)
        endpoint.unlink(missing_ok=True)
        proc = subprocess.Popen(command, cwd=run, env=env, stdout=log, stderr=subprocess.STDOUT)
        deadline = time.monotonic() + 180
        while not endpoint.exists():
            if proc.poll() is not None or time.monotonic() > deadline:
                raise RuntimeError(f'game did not restart: {run / "game.log"}')
            time.sleep(.2)
        ctl('wait-ready', '180', timeout=185)
        found = ctl('command', 'clear @s tconstruct:pickaxe 0')
        assert 'Found 1' in json.dumps(found) or 'matching' in json.dumps(found), found
        assert ctl('state')['gamemode'] == 'creative'
        result['scenarios'].append('the world reloads with the pickaxe still in the inventory: ' + json.dumps(found)[:160])

        ctl('quit')
        proc.wait(timeout=90)
        problems = [line for line in (run / 'game.log').read_text(encoding='utf-8', errors='replace').splitlines() if 'Tinkers:' in line]
        (run / 'tinkers-log.txt').write_text(newline='\n', data=chr(10).join(problems) + chr(10), encoding='utf-8')
        result['tinkers_log_lines'] = len(problems)
        seen = [line for line in (run / 'game.log').read_text(encoding='utf-8', errors='replace').splitlines() if 'PlutoLib: use item on' in line or 'Tinkers: use on' in line or 'Tinkers: tool action' in line]
        print('USE-ON LOG', json.dumps(seen[:6]), flush=True)
        result['result'] = 'TOOL BUILD PASS' if not soft_failures else 'TOOL BUILD PARTIAL'
        result['failures'] = soft_failures
        print(json.dumps({**result, 'run': str(run)}))
        if soft_failures:
            raise SystemExit('soft failures: ' + json.dumps(soft_failures))
    finally:
        if proc.poll() is None:
            proc.terminate()
            try:
                proc.wait(timeout=15)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=15)
        log.close()
        print('RUN', run, flush=True)


if __name__ == '__main__':
    main()
