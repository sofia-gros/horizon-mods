"""Exercise the mod Creative tab, a material tool, and equipment acquisition."""
import argparse
import ctypes
import re
import subprocess
import time
from pathlib import Path
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
    # Creative tab headers use the window event path, not click-slot.
    tree = subprocess.check_output(['xwininfo', '-root', '-tree'], text=True)
    window = int(re.search(r'(0x[0-9a-f]+) "Horizon"', tree).group(1), 16)
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
    x.XFlush(display); time.sleep(.15)
    xtest.XTestFakeButtonEvent(display, 1, 1, 0); x.XFlush(display); time.sleep(.1)
    if drag_y is not None:
        x.XWarpPointer(display, 0, window, 0, 0, 0, 0, x_pos, drag_y)
        x.XFlush(display); time.sleep(.15)
    xtest.XTestFakeButtonEvent(display, 1, 0, 0); x.XFlush(display)
    x.XCloseDisplay(display)

try:
    g.ready()
    for cmd in ('gamemode creative @s', 'clear @s', 'tp @s 0.5 71 0.5', 'setblock 0 71 2 minecraft:stone'):
        g.call('command', cmd)
    g.call('open-inventory'); g.call('wait', '10')
    # TConstruct uses the otherwise-hidden top-row hotbar position.
    screen_click(596, 105); g.call('wait', '10')
    g.call('screenshot', str(run / 'pluto-creative-tab.png'))
    # The first five items have no materials; source-ordered pickaxes follow.
    screen_click(499, 188); g.call('wait', '5')
    carried = g.call('inventory')['carried'] or {}
    assert carried.get('item') == 'tconstruct:pickaxe', carried
    assert 'tic_max_damage:60' in carried.get('components', ''), carried
    screen_click(319, 376); g.call('wait', '5')
    # Scroll to the source armor section, then select a visible plate helmet.
    screen_click(649, 186, drag_y=350); g.call('wait', '10')
    nodes = g.call('ui')['widgets']
    helmet = next((node for node in nodes if (node.get('stack') or {}).get('item') == 'tconstruct:plate_helmet'), None)
    assert helmet is not None, [(n.get('id'), (n.get('stack') or {}).get('item')) for n in nodes]
    index = int(helmet['id'].split('-')[1]) - 900
    screen_click(285 + (17 + (index % 9) * 18) * 2, 136 + (26 + (index // 9) * 18) * 2)
    g.call('wait', '5')
    assert (g.call('inventory')['carried'] or {}).get('item') == 'tconstruct:plate_helmet'
    screen_click(355, 376); g.call('wait', '5'); g.call('close')
    g.call('hotbar', '1'); g.call('command', 'gamemode survival @s'); g.call('wait', '5')
    g.call('break', '0', '71', '2', '--timeout', '30', timeout=35)
    held = g.call('inventory')['held'] or {}
    assert held.get('item') == 'tconstruct:pickaxe' and held.get('damage') == 1, held
    g.call('open-inventory'); g.call('wait', '5')
    g.call('click-slot', '1'); g.call('click-slot', '500'); g.call('wait', '5')
    assert g.call('inventory')['armor'][0].get('item') == 'tconstruct:plate_helmet'
    g.call('close'); g.call('save'); g.stop()
    print('PASS Creative tab, material pickaxe acquisition and mining, helmet acquisition and placement', flush=True)
finally:
    g.cleanup()
