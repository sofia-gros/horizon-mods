import argparse, json, sys, time
from pathlib import Path
from harness import Game
p = argparse.ArgumentParser()
for k in ('game', 'ctl', 'resources'):
    p.add_argument('--' + k, type=Path, required=True)
p.add_argument('--run-dir', type=Path, default=Path('target/pb-station-batch'))
a = p.parse_args()
run = a.run_dir.resolve()
games = []

def cmd(g, s):
    r = g.call('command', s)
    g.call('wait', '10')
    print('command', s, r, flush=True)
try:
    h = Game(a, run, 'Player', resume=True)
    games.append(h)
    h.ready()
    port = h.call('lan', 'open', '0', 'commands')['port']
    q = Game(a, run, 'Guest', server='127.0.0.1:' + str(port), resume=True)
    games.append(q)
    q.ready()
    for s in ('gamemode creative @a', 'kill @e[type=minecraft:zombie,tag=tconstruct_upgrade_test]', 'clear @a', 'tp @s -0.5 71 0.5', 'tp Guest 1.5 71 0.5', 'setblock 0 71 2 minecraft:air', 'setblock 0 71 2 minecraft:chest'):
        cmd(h, s)
    q.call('wait', '20')
    status = q.call('status')
    assert status['creative'] and status['player']['position'] == [1.5, 71.0, 0.5]
    print('chest', q.call('interact', '0', '71', '2'), flush=True)
    q.call('wait', '40')
    assert q.call('menu')['screen'] == 'inventory'
    print('chest-menu opened', flush=True)
    q.call('close')
    cmd(h, 'setblock 0 71 2 minecraft:air')
    cmd(h, 'setblock 0 71 2 tconstruct:tinker_station')
    q.call('wait', '20')
    print('station', q.call('interact', '0', '71', '2'), flush=True)
    q.call('wait', '40')
    assert q.call('menu')['screen'] == 'inventory'
    print('station-menu opened', flush=True)
    for g in reversed(games):
        g.stop()
finally:
    for g in reversed(games):
        g.cleanup()
