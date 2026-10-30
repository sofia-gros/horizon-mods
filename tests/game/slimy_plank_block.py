"""Check one source slimewood plank block and its non-fuel item."""
import argparse, json, tempfile, time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
p.add_argument('--game',type=Path,required=True);p.add_argument('--ctl',type=Path,required=True)
p.add_argument('--resources',type=Path,required=True);p.add_argument('--in-frame',action='store_true')
a=p.parse_args()
facts=json.loads((a.resources/'facts.json').read_text()); gameplay=json.loads((a.resources/'gameplay-facts.json').read_text())
items=json.loads((a.resources/'items.json').read_text()); registries=json.loads((a.resources/'registries.json').read_text())
sound=gameplay['states'][facts['defaults']['minecraft:slime_block']]['sound']
for name,color in [('greenheart',8375321),('skyroot',5013401),('bloodshroom',10040115),('enderbark',1644825)]:
    item='tconstruct:'+name+'_planks'; row=gameplay['states'][facts['defaults'][item]]
    assert item in registries['minecraft:item']['entries'] and 'minecraft:cooking_fuel' not in items[item]
    assert row['hardness']==2 and row['explosion_resistance']==3 and not row['requires_tool']
    assert row['map_color']==color and row['sound']==sound
run=Path(tempfile.mkdtemp(prefix='slimy-plank-',dir=Path('target').resolve()))
print('run',run,flush=True);g=Game(a,run,'Player',in_frame=a.in_frame)
try:
    g.ready()
    for cmd in ('fill -2 70 -1 4 70 5 minecraft:stone','tp @s -0.5 71 0.5','clear @s','give @s tconstruct:greenheart_planks 1'):
        g.call('command',cmd)
    g.call('wait','20');time.sleep(.5)
    g.call('place','0','71','2','--item','tconstruct:greenheart_planks')
    assert g.call('block','0','71','2')['block']=='tconstruct:greenheart_planks'
    g.call('wait','20');g.call('screenshot',str(run/'greenheart.png'))
    g.call('save');g.stop();g.cleanup()
    g=Game(a,run,'Player',in_frame=a.in_frame,resume=True);g.ready()
    assert g.call('block','0','71','2')['block']=='tconstruct:greenheart_planks'
    for cmd in ('gamemode survival @s', 'clear @s', 'give @s minecraft:diamond_axe 1'):
        g.call('command',cmd)
    g.call('wait','20');g.call('select','minecraft:diamond_axe')
    g.call('break','0','71','2','--timeout','30',timeout=35)
    g.call('command','tp @s 0.5 71 2.5');g.call('wait','60')
    for _ in range(40):
        if any(x.get('item')=='tconstruct:greenheart_planks' for x in g.call('inventory')['hotbar']): break
        time.sleep(.25)
    else: raise AssertionError('missing plank drop')
    for cmd in ('gamemode creative @s','tp @s -0.5 71 0.5','clear @s','setblock 0 71 2 minecraft:furnace','give @s minecraft:iron_ore 1','give @s tconstruct:greenheart_planks 1'):
        g.call('command',cmd)
    g.call('wait','20');g.call('interact','0','71','2');g.call('wait','10')
    for slot in (0,300,1,301): g.call('click-slot',str(slot));g.call('wait','5')
    g.call('wait','100')
    slots={x['id']:x.get('stack') for x in g.call('ui')['widgets'] if x['id'].startswith('slot-')}
    assert slots['slot-301'] is None,slots
    assert (g.call('inventory')['carried'] or {}).get('item')=='tconstruct:greenheart_planks'
    assert slots['slot-302'] is None,slots
    print('PASS original plank placement, save/reload, drop and non-fuel',flush=True);g.stop()
finally:
    g.cleanup()
