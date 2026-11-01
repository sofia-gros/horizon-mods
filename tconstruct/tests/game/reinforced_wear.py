"""Representative ordinary wear on tools, armor and bows.

Level V is command-supplied to make the small sample useful; actual creation is
covered separately. Source probabilities and Dense ordering are unit-tested.
"""
import argparse,json
from pathlib import Path
from harness import Game
p=argparse.ArgumentParser()
for n in ('game','ctl','resources','run-dir'):p.add_argument('--'+n,type=Path,required=True)
p.add_argument('--in-frame',action='store_true');a=p.parse_args();run=a.run_dir.resolve();assert run.is_relative_to((Path(__file__).resolve().parents[2]/'target').resolve());g=Game(a,run,'Player',in_frame=a.in_frame,resume=True)
pack=g.root/'saves/Probe/datapacks/reinforced-dense-test';created=False
def cmd(s):g.call('command',s);g.call('wait','6')
def damage(stack):return stack.get('damage',0)
def data(kind,reinforced):
 parts='["tconstruct:flint","tconstruct:wood","tconstruct:wood"]' if kind=='pickaxe' else ('["tconstruct:wood","tconstruct:wood","tconstruct:wood","tconstruct:string"]' if kind=='longbow' else '["tconstruct:wood","tconstruct:wood","tconstruct:string"]')
 up=',tic_upgrades:{"tconstruct:reinforced":20}' if reinforced else ''
 return f'tconstruct:{kind}[minecraft:custom_data={{tic_materials:{parts},tic_max_damage:100,tic_draw_speed:2.0f,tic_velocity:1.0f,tic_accuracy:1.0f{up}}},minecraft:max_damage=100]'
def setup(kind,reinforced):
 for c in ('gamemode creative @s','clear @s','effect clear @s','effect give @s minecraft:instant_health 1 4 true','kill @e[tag=gem_arrow_target]','tp @s 0.5 71 0.5','fill -2 71 -1 2 72 3 minecraft:air','item replace entity @s weapon.mainhand with '+data(kind,reinforced),'give @s minecraft:arrow 32','gamemode survival @s'):cmd(c)
 g.call('hotbar','1');g.call('look','0','-60');g.call('wait','10')
def shoot(kind):
 g.call('mouse','right','down');g.call('wait','35');g.call('mouse','right','up')
 if kind=='crossbow':g.call('wait','4');g.call('mouse','right','down');g.call('wait','2');g.call('mouse','right','up')
 g.call('wait','6')
def bow(kind,reinforced):
 setup(kind,reinforced);before=g.call('inventory')['counts'].get('minecraft:arrow',0)
 for _ in range(5):shoot(kind)
 inv=g.call('inventory');assert before-inv['counts'].get('minecraft:arrow',0)==5,(kind,inv);return damage(inv['held'])
def pick(reinforced):
 setup('pickaxe',reinforced)
 for _ in range(5):
  cmd('gamemode creative @s');cmd('setblock 0 71 2 minecraft:stone');cmd('gamemode survival @s');g.call('wait','10');g.call('break','0','71','2','--timeout','20',timeout=25)
 return damage(g.call('inventory')['held'])
def armor(reinforced):
 up=',tic_upgrades:{"tconstruct:reinforced":20}' if reinforced else ''
 item='tconstruct:plate_helmet[minecraft:max_damage=100,minecraft:custom_data={tic_armor:2.0f,tic_armor_toughness:0.0f,tic_materials:["tconstruct:iron","tconstruct:iron"],tic_max_damage:100'+up+'}]'
 for c in ('gamemode creative @s','effect clear @s','effect give @s minecraft:instant_health 1 4 true','item replace entity @s armor.chest with minecraft:air','item replace entity @s armor.legs with minecraft:air','item replace entity @s armor.feet with minecraft:air','item replace entity @s armor.head with '+item,'gamemode survival @s'):cmd(c)
 for _ in range(5):g.call('wait','30');cmd('effect give @s minecraft:instant_health 1 4 true');cmd('damage @s 4 minecraft:mob_attack')
 return damage(g.call('inventory')['armor'][0])
try:
 g.ready()
 if g.call('status')['screen']=='dead':g.call('respawn');g.call('wait','20')
 for kind in ('longbow','crossbow'):
  base=bow(kind,False);protected=bow(kind,True);print('WEAR',kind,base,protected,flush=True);assert base==5 and protected<base,(kind,base,protected)
 base=pick(False);protected=pick(True);print('WEAR pickaxe',base,protected,flush=True);assert base==5 and protected<base,(base,protected)
 base=armor(False);protected=armor(True);print('WEAR helmet',base,protected,flush=True);assert base==5 and protected<base,(base,protected)
 if pack.exists():raise RuntimeError('test datapack path exists')
 (pack/'data/tconstruct/tinkering/materials/traits').mkdir(parents=True);created=True
 (pack/'pack.mcmeta').write_text('{"pack":{"pack_format":15,"description":"temporary reinforced Dense test"}}')
 (pack/'data/tconstruct/tinkering/materials/traits/flint.json').write_text(json.dumps({'default':[{'name':'tconstruct:dense','level':1}]}))
 cmd('gamemode creative @s');cmd('reload');cmd('datapack enable "file/reinforced-dense-test"');g.call('wait','50')
 combined=pick(True);print('WEAR Dense+Reinforced pickaxe',combined,flush=True);assert combined<5,combined
 cmd('gamemode creative @s');cmd('datapack disable "file/reinforced-dense-test"');g.call('save');g.stop();print('PASS representative Reinforced wear',flush=True)
finally:
 g.cleanup()
 if created:
  (pack/'data/tconstruct/tinkering/materials/traits/flint.json').unlink(missing_ok=True);(pack/'pack.mcmeta').unlink(missing_ok=True)
  for sub in ('data/tconstruct/tinkering/materials/traits','data/tconstruct/tinkering/materials','data/tconstruct/tinkering','data/tconstruct','data',''):(pack/sub).rmdir()
