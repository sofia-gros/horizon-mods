"""Place a faucet on each unformed drain without sneaking.

Run in a disposable world. The game-control `place` command uses normal item
interaction; unlike setblock, it detects failures in use_hand and consumes the
item in survival. Existing fluid_containers_direct.py covers drain transfers.
"""
import argparse
from pathlib import Path
from harness import Game

p = argparse.ArgumentParser()
for key in ('game', 'ctl', 'resources'):
    p.add_argument('--' + key, type=Path, required=True)
p.add_argument('--run-dir', type=Path, default=Path('target/faucet-drain-placement'))
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
run = a.run_dir.resolve()
run.mkdir(parents=True, exist_ok=True)
g = Game(a, run, 'Player', in_frame=a.in_frame, resume=False)

try:
    g.ready()
    for command in ('gamemode creative @s', 'clear @s', 'fill -3 70 -3 3 74 5 minecraft:air',
                    'fill -3 69 -3 3 69 5 minecraft:stone', 'tp @s 0.5 70 4.5',
                    'give @s tconstruct:seared_faucet 3'):
        g.call('command', command)
    for drain in ('seared_drain', 'scorched_drain'):
        g.call('command', f'setblock 0 71 2 tconstruct:{drain}[facing=south]')
        g.call('command', 'setblock 0 71 3 minecraft:air')
        g.call('command', 'gamemode survival @s')
        before = sum(s.get('count', 0) for s in g.call('inventory')['hotbar'] if s.get('item') == 'tconstruct:seared_faucet')
        g.call('place', '0', '71', '3', '--item', 'tconstruct:seared_faucet')
        g.call('wait', '10')
        block = g.call('block', '0', '71', '3')
        assert block.get('block') == 'tconstruct:seared_faucet', (drain, block)
        assert 'facing=south' in block['state'], (drain, block)
        after = sum(s.get('count', 0) for s in g.call('inventory')['hotbar'] if s.get('item') == 'tconstruct:seared_faucet')
        assert after == before - 1, (drain, before, after)
        g.call('command', 'gamemode creative @s')
    g.stop()
    print('PASS normal-click faucet placement on seared and scorched drains', flush=True)
finally:
    g.cleanup()
