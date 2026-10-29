"""Exercise the source stone-tool recycling transaction in the GUI."""
import argparse, tempfile, time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
p.add_argument('--game',type=Path,required=True)
p.add_argument('--ctl',type=Path,required=True)
p.add_argument('--resources',type=Path,required=True)
p.add_argument('--in-frame',action='store_true')
p.add_argument('--damage',type=int,default=0)
p.add_argument('--enchanted',action='store_true')
p.add_argument('--quick',action='store_true')
p.add_argument('--flint-steel',action='store_true')
p.add_argument('--iron-choice',action='store_true')
p.add_argument('--crossbow',action='store_true')
p.add_argument('--reload-after-select',action='store_true')
p.add_argument('--reselect-after-reload',action='store_true')
p.add_argument('--replace-pattern-after-reload',action='store_true')
a=p.parse_args()
run=Path(tempfile.mkdtemp(prefix='recycling-preview-',dir=Path('target').resolve()))
print('run',run,flush=True)
g=Game(a,run,'Player',in_frame=a.in_frame)
def click(slot):
    g.call('click-slot',str(slot));g.call('wait','3')
try:
    g.ready()
    tool = 'minecraft:crossbow' if a.crossbow else 'minecraft:flint_and_steel' if a.flint_steel else 'minecraft:stone_axe'
    pattern = 'Rod' if a.crossbow else ('Ingot' if a.iron_choice else 'Shard') if a.flint_steel else 'Block'
    result_item = 'minecraft:stick' if a.crossbow else ('minecraft:iron_ingot' if a.iron_choice else 'minecraft:flint') if a.flint_steel else 'minecraft:cobblestone'
    alternate = 'minecraft:flint' if a.iron_choice else 'minecraft:iron_ingot'
    for c in ('clear @s','tp @s 0.5 71 0.5','setblock 0 71 2 tconstruct:part_builder',
              'give @s tconstruct:venom_bottle 1',f'give @s {tool}[minecraft:damage={a.damage}] 1'):
        g.call('command',c)
    if a.enchanted:
        g.call('key','2','tap');g.call('wait','3')
        enchant = g.call('command','enchant @s minecraft:efficiency 1')
        print('enchant',enchant,flush=True)
        assert any('Applied enchantment' in message for message in enchant.get('chat', [])), enchant
        g.call('key','1','tap')
    g.call('interact','0','71','2')
    click(0);click(300);click(1);click(301)
    for _ in range(120):
        node=next((n for n in g.call('ui')['widgets'] if n.get('label')==pattern and n.get('enabled')),None)
        if node: break
        time.sleep(.2)
    assert node, g.call('ui')
    g.call('click',node['id']);g.call('wait','20')
    if a.reload_after_select:
        g.call('command','reload');g.call('wait','60')
        if a.replace_pattern_after_reload:
            click(300); click(2); click(2); click(300); g.call('wait','20')
        if a.reselect_after_reload:
            fresh=next(n for n in g.call('ui')['widgets'] if n.get('label')==pattern and n.get('enabled'))
            g.call('click',fresh['id']);g.call('wait','20')
    g.call('screenshot',str(run/'recycling-choice.png'))
    original_count = 6 * (465 - a.damage) // 465 if a.crossbow else 2 * (64 - a.damage) // 64 if a.flint_steel else 3 * (131 - a.damage) // 131
    unsupported_random = False
    expected = 0 if a.enchanted or unsupported_random else min(3, original_count) if a.crossbow else min(1, original_count) if a.flint_steel else original_count
    if a.quick:
        g.call('click-slot','322','shift');g.call('wait','10')
        result = next((x for x in g.call('inventory')['hotbar'] if x.get('item') == result_item), {})
    else:
        click(322)
        result = g.call('menu').get('carried') or {}
    if expected == 0:
        assert not result, result
        rows = {r['slot']: r['stack'] for r in g.call('menu')['slots'] if r.get('stack')}
        assert rows.get(300, {}).get('item') == 'tconstruct:venom_bottle' and rows.get(301, {}).get('item') == tool, rows
        print('PASS unsupported random remainder remains unavailable' if unsupported_random else 'PASS source damage/enchantment rejects output without consuming',flush=True)
        g.stop()
        raise SystemExit(0)
    assert result.get('item') == result_item and result.get('count') == expected, result
    rows = {r['slot']: r['stack'] for r in g.call('menu')['slots'] if r.get('stack')}
    assert rows.get(300, {}).get('item') == 'minecraft:glass_bottle' and 301 not in rows, rows
    if a.flint_steel and original_count > 1 and a.replace_pattern_after_reload and not a.reselect_after_reload:
        inventory = g.call('inventory')['hotbar']
        candidates = [(x.get('item'), x.get('count')) for x in inventory if x.get('item') in ('minecraft:flint', 'minecraft:iron_ingot')]
        assert candidates in [[('minecraft:flint', 1)], [('minecraft:iron_ingot', 1)]], candidates
        alternate = candidates[0][0]
    if not a.quick: click(1)
    g.call('screenshot',str(run/'recycled.png'))
    if a.flint_steel and original_count > 1:
        assert sum(x.get('count', 0) for x in g.call('inventory')['hotbar'] if x.get('item') == alternate) == (2 if alternate == result_item and a.replace_pattern_after_reload and not a.reselect_after_reload else 1), g.call('inventory')
    if a.crossbow:
        items = g.call('inventory')['hotbar']
        leftovers = [(x.get('item'), x.get('count')) for x in items if x.get('item') in ('minecraft:string', 'minecraft:iron_ingot')]
        assert leftovers in [[('minecraft:string', 2)], [('minecraft:iron_ingot', 1)]], leftovers
    g.call('close');g.call('save');g.stop();g.cleanup()
    g=Game(a,run,'Player',in_frame=a.in_frame,resume=True);g.ready()
    assert sum(x.get('count', 0) for x in g.call('inventory')['hotbar'] if x.get('item') == result_item) == (2 if a.flint_steel and alternate == result_item and a.replace_pattern_after_reload and not a.reselect_after_reload else expected)
    if a.flint_steel and original_count > 1:
        assert sum(x.get('count', 0) for x in g.call('inventory')['hotbar'] if x.get('item') == alternate) == (2 if alternate == result_item and a.replace_pattern_after_reload and not a.reselect_after_reload else 1)
    if a.crossbow:
        items = g.call('inventory')['hotbar']
        assert any(x.get('item') == leftovers[0][0] and x.get('count') == leftovers[0][1] for x in items), items
    print('PASS source tool recycling, bottle and save/reload',flush=True)
    g.stop()
finally:
    g.cleanup()
