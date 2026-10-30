"""Exercise the mod Creative tab, a material tool, and equipment acquisition."""
import argparse
import ctypes
import re
import subprocess
import time
from pathlib import Path
import sys, json
from harness import Game
p = argparse.ArgumentParser()
p.add_argument('--game', type=Path, required=True)
p.add_argument('--ctl', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True)
p.add_argument('--run-dir', type=Path, default=Path('target/pb-station-batch'))
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
run = a.run_dir.resolve()
run.mkdir(parents=True, exist_ok=True)
g = Game(a, run, 'Player', in_frame=a.in_frame, resume=(run / 'Player').exists())

def screen_click(x_pos, y_pos, drag_y=None):
    tree = subprocess.check_output(['xwininfo', '-root', '-tree'], text=True)
    window = int(re.search('(0x[0-9a-f]+) "Horizon"', tree).group(1), 16)
    x = ctypes.CDLL('libX11.so.6')
    xtest = ctypes.CDLL('libXtst.so.6')
    x.XOpenDisplay.restype = ctypes.c_void_p
    display = x.XOpenDisplay(None)
    x.XSetInputFocus.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int, ctypes.c_ulong]
    x.XWarpPointer.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_ulong, ctypes.c_int, ctypes.c_int, ctypes.c_uint, ctypes.c_uint, ctypes.c_int, ctypes.c_int]
    x.XFlush.argtypes = [ctypes.c_void_p]
    x.XCloseDisplay.argtypes = [ctypes.c_void_p]
    xtest.XTestFakeButtonEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_int, ctypes.c_ulong]
    x.XSetInputFocus(display, window, 1, 0)
    x.XWarpPointer(display, 0, window, 0, 0, 0, 0, x_pos, y_pos)
    x.XFlush(display)
    time.sleep(0.15)
    xtest.XTestFakeButtonEvent(display, 1, 1, 0)
    x.XFlush(display)
    time.sleep(0.1)
    if drag_y is not None:
        x.XWarpPointer(display, 0, window, 0, 0, 0, 0, x_pos, drag_y)
        x.XFlush(display)
        time.sleep(0.15)
    xtest.XTestFakeButtonEvent(display, 1, 0, 0)
    x.XFlush(display)
    x.XCloseDisplay(display)

def cmd(x):
    r = g.call('command', x)
    g.call('wait', '6')
    return r
try:
    g.ready()
    if g.call('status')['screen'] == 'dead':
        g.call('respawn')
        g.call('wait', '20')
    for x in ('gamemode creative @s', 'clear @s', 'effect clear @s', 'effect give @s minecraft:instant_health 1 4 true', 'kill @e[type=minecraft:zombie]', 'tp @s 0.5 71 0.5'):
        cmd(x)
    g.call('open-inventory')
    g.call('wait', '10')
    screen_click(596, 105)
    g.call('wait', '10')
    screen_click(571, 332)
    g.call('wait', '8')
    picked = g.call('inventory')['carried'] or {}
    print('creative steel', json.dumps(picked), flush=True)
    assert picked.get('item') == 'tconstruct:pickaxe' and 'tconstruct:steel' in picked.get('components', '') and ('tic_max_damage:1046' in picked.get('components', ''))
    g.call('screenshot', str(run / 'trait-stat-creative.png'))
    screen_click(319, 376)
    g.call('wait', '5')
    g.call('close')
    g.call('hotbar', '1')
    g.call('wait', '5')
    held = g.call('inventory')['held']
    print('held', json.dumps(held), flush=True)
    cmd('gamemode survival @s')
    cmd('summon minecraft:zombie 1.5 71 0.5 {NoAI:1b}')
    g.call('wait', '30')
    g.call('hurtlog')
    entities = g.call('entities', '8', 'minecraft:zombie')['entities']
    g.call('attack', str(entities[0]['id']))
    log = g.call('hurtlog')
    print('steel attack', json.dumps(log), flush=True)
    amount = log['players'][0]['attacks'][-1]['amount']
    assert abs(amount - 4.9) < 0.01
    cmd('gamemode creative @s')
    cmd('kill @e[type=minecraft:zombie]')
    cmd('clear @s')
    cmd('effect give @s minecraft:instant_health 1 4 true')
    helmet = 'tconstruct:plate_helmet[minecraft:max_damage=382,minecraft:custom_data={tic_armor:2.0f,tic_armor_toughness:4.0f,tic_knockback_resistance:0.0f,tic_materials:["tconstruct:steel","tconstruct:steel"],tic_max_damage:382}]'
    for idx, item in enumerate([helmet, 'minecraft:air', helmet]):
        cmd('item replace entity @s armor.head with ' + item)
        cmd('gamemode survival @s')
        g.call('wait', '25')
        g.call('hurtlog')
        cmd('damage @s 4 minecraft:mob_attack')
        entry = g.call('hurtlog')['players'][0]['hurts'][-1]
        print('armor cycle', idx, json.dumps(entry), flush=True)
        assert abs(entry['toughness'] - (0.0 if idx == 1 else 4.0)) < 0.001
        cmd('gamemode creative @s')
        cmd('effect give @s minecraft:instant_health 1 4 true')
    g.stop()
    print('PASS creative steel attack and armor', flush=True)
finally:
    g.cleanup()
