"""Compare 1.20.1 crafting-container behavior through the Part Builder."""
import argparse, json, tempfile, time
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
p.add_argument('--game',type=Path,required=True);p.add_argument('--ctl',type=Path,required=True)
p.add_argument('--resources',type=Path,required=True);p.add_argument('--in-frame',action='store_true')
p.add_argument('--item',choices=['water_bucket','potion','honey_bottle'],required=True)
a=p.parse_args();run=Path(tempfile.mkdtemp(prefix='source-crafting-remainder-',dir=Path('target').resolve()))
print('run',run,flush=True);g=Game(a,run,'Player',in_frame=a.in_frame)
def click(slot):g.call('click-slot',str(slot));g.call('wait','3')
try:
    g.ready(); pack=g.root/'saves/Probe/datapacks/source-remainder'
    path=pack/'data/tconstruct/recipes/tools/materials/flint/flint.json';path.parent.mkdir(parents=True)
    (pack/'pack.mcmeta').write_text('{"pack":{"pack_format":15,"description":"source remainder"}}')
    recipe=json.loads((a.resources/'tconstruct-original-pack/data/tconstruct/recipes/tools/materials/flint/flint.json').read_text())
    recipe['ingredient']={'item':'minecraft:'+a.item};path.write_text(json.dumps(recipe))
    if a.item == 'honey_bottle':
        honey=pack/'data/tconstruct/recipes/tools/materials/honey.json'; honey.parent.mkdir(parents=True,exist_ok=True)
        honey.write_text('{"type":"minecraft:crafting_shapeless","ingredients":[{"item":"minecraft:stone"}],"result":{"item":"minecraft:stone"}}')
    g.call('command','reload');g.call('wait','60')
    for cmd in ('clear @s','tp @s 0.5 71 0.5','setblock 0 71 2 tconstruct:part_builder',
                'give @s tconstruct:pattern 1','give @s minecraft:'+a.item+(' 2' if a.item == 'honey_bottle' else ' 1')):
        g.call('command',cmd)
    g.call('interact','0','71','2');click(0);click(300);click(1);click(301)
    for _ in range(120):
        node=next((n for n in g.call('ui')['widgets'] if n.get('label')=='Tool Handle' and n.get('enabled')),None)
        if node:break
        time.sleep(.2)
    assert node,g.call('menu')
    g.call('click',node['id']);g.call('wait','20');click(322)
    assert (g.call('menu').get('carried') or {}).get('item')=='tconstruct:tool_handle'
    slots={s['slot']:s['stack'] for s in g.call('menu')['slots'] if s.get('stack')}
    if a.item=='water_bucket':
        assert slots.get(301,{}).get('item')=='minecraft:bucket',slots
    elif a.item=='honey_bottle':
        assert slots.get(301,{}).get('item')=='minecraft:honey_bottle' and slots[301]['count']==1,slots
        assert any(x.get('item')=='minecraft:glass_bottle' for x in g.call('inventory')['hotbar'])
    else:
        assert 301 not in slots,slots
    print('PASS source crafting remainder',a.item,flush=True);g.stop()
finally:
    g.cleanup()
