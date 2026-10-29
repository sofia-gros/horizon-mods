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
a=p.parse_args()
run=Path(tempfile.mkdtemp(prefix='recycling-preview-',dir=Path('target').resolve()))
print('run',run,flush=True)
g=Game(a,run,'Player',in_frame=a.in_frame)
def click(slot):
    g.call('click-slot',str(slot));g.call('wait','3')
try:
    g.ready()
    for c in ('clear @s','tp @s 0.5 71 0.5','setblock 0 71 2 tconstruct:part_builder',
              'give @s tconstruct:venom_bottle 1',f'give @s minecraft:stone_axe[minecraft:damage={a.damage}] 1'):
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
        node=next((n for n in g.call('ui')['widgets'] if n.get('label')=='Block' and n.get('enabled')),None)
        if node: break
        time.sleep(.2)
    assert node, g.call('ui')
    g.call('click',node['id']);g.call('wait','20')
    g.call('screenshot',str(run/'recycling-choice.png'))
    expected = 0 if a.enchanted else 3 * (131 - a.damage) // 131
    if a.quick:
        g.call('click-slot','322','shift');g.call('wait','10')
        result = next((x for x in g.call('inventory')['hotbar'] if x.get('item') == 'minecraft:cobblestone'), {})
    else:
        click(322)
        result = g.call('menu').get('carried') or {}
    if expected == 0:
        assert not result, result
        rows = {r['slot']: r['stack'] for r in g.call('menu')['slots'] if r.get('stack')}
        assert rows.get(300, {}).get('item') == 'tconstruct:venom_bottle' and rows.get(301, {}).get('item') == 'minecraft:stone_axe', rows
        print('PASS source damage boundary rejects empty output without consuming',flush=True)
        g.stop()
        raise SystemExit(0)
    assert result.get('item') == 'minecraft:cobblestone' and result.get('count') == expected, result
    rows = {r['slot']: r['stack'] for r in g.call('menu')['slots'] if r.get('stack')}
    assert rows.get(300, {}).get('item') == 'minecraft:glass_bottle' and 301 not in rows, rows
    if not a.quick: click(1)
    g.call('close');g.call('save');g.stop();g.cleanup()
    g=Game(a,run,'Player',in_frame=a.in_frame,resume=True);g.ready()
    assert any(x.get('item') == 'minecraft:cobblestone' and x.get('count') == expected for x in g.call('inventory')['hotbar'])
    print('PASS source stone axe recycling, bottle and save/reload',flush=True)
    g.stop()
finally:
    g.cleanup()
