"""Exercise the source venom bottle's registered drinking behavior."""
import argparse, tempfile, time
from pathlib import Path
from harness import Game
p = argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True)
p.add_argument('--ctl', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True)
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
run = Path(tempfile.mkdtemp(prefix='venom-bottle-', dir=Path('target').resolve()))
print('run', run, flush=True)
g = Game(a, run, 'Player', in_frame=a.in_frame)
try:
    g.ready()
    for command in ('clear @s', 'gamemode survival @s', 'effect clear @s',
                    'tp @s 0.5 71 0.5', 'give @s tconstruct:venom_bottle 1'):
        g.call('command', command)
    g.call('wait', '20')
    g.call('key', 'e', 'tap'); g.call('wait', '10')
    g.call('screenshot', str(run / 'venom-bottle.png'))
    g.call('key', 'e', 'tap'); g.call('wait', '10')
    g.call('key', '1', 'tap')
    initial = g.call('status')['player']['health']
    g.call('mouse', 'right', 'down'); g.call('wait', '40'); g.call('mouse', 'right', 'up')
    g.call('wait', '80')
    inventory = g.call('inventory')['hotbar']
    health = g.call('status')['player']['health']
    assert any(s.get('item') == 'minecraft:glass_bottle' for s in inventory), inventory
    assert not any(s.get('item') == 'tconstruct:venom_bottle' for s in inventory), inventory
    assert health < initial, (initial, health)
    strength = g.call('command', 'effect clear @s minecraft:strength')
    print('strength clear', strength, flush=True)
    assert any('strength' in message.lower() and 'removed' in message.lower() for message in strength.get('chat', [])), strength
    g.call('save'); g.stop()
    print('PASS venom bottle drink, poison damage and glass bottle', flush=True)
finally:
    g.cleanup()
