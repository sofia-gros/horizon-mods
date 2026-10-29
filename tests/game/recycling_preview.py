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
a=p.parse_args()
run=Path(tempfile.mkdtemp(prefix='recycling-preview-',dir=Path('target').resolve()))
print('run',run,flush=True)
g=Game(a,run,'Player',in_frame=a.in_frame)
def click(slot):
    g.call('click-slot',str(slot));g.call('wait','3')
try:
    g.ready()
    tool = 'minecraft:flint_and_steel' if a.flint_steel else 'minecraft:stone_axe'
    pattern = 'Shard' if a.flint_steel else 'Block'
    result_item = 'minecraft:flint' if a.flint_steel else 'minecraft:cobblestone'
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
    g.call('screenshot',str(run/'recycling-choice.png'))
    original_count = 2 * (64 - a.damage) // 64 if a.flint_steel else 3 * (131 - a.damage) // 131
    unsupported_random = a.flint_steel and original_count > 1
    expected = 0 if a.enchanted or unsupported_random else min(1, original_count) if a.flint_steel else original_count
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
    if not a.quick: click(1)
    g.call('close');g.call('save');g.stop();g.cleanup()
    g=Game(a,run,'Player',in_frame=a.in_frame,resume=True);g.ready()
    assert any(x.get('item') == result_item and x.get('count') == expected for x in g.call('inventory')['hotbar'])
    print('PASS source tool recycling, bottle and save/reload',flush=True)
    g.stop()
finally:
    g.cleanup()
