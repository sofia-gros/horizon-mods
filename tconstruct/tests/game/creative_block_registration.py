"""Acquire and place a registered block from the TConstruct Creative group."""
import argparse
import ctypes
import re
import subprocess
import time
from pathlib import Path
from harness import Game

p = argparse.ArgumentParser()
for name in ('game', 'ctl', 'resources'):
    p.add_argument('--' + name, type=Path, required=True)
p.add_argument('--run-dir', type=Path, default=Path('target/pb-station-batch'))
p.add_argument('--in-frame', action='store_true')
a = p.parse_args()
run = a.run_dir.resolve()
g = Game(a, run, 'Player', in_frame=a.in_frame, resume=True)

def screen_click(x_pos, y_pos):
    tree = subprocess.check_output(['xwininfo', '-root', '-tree'], text=True)
    window = int(re.search(r'(0x[0-9a-f]+) "Horizon"', tree).group(1), 16)
    x = ctypes.CDLL('libX11.so.6'); xtest = ctypes.CDLL('libXtst.so.6')
    x.XOpenDisplay.restype = ctypes.c_void_p; display = x.XOpenDisplay(None)
    x.XSetInputFocus.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int, ctypes.c_ulong]
    x.XWarpPointer.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_ulong, ctypes.c_int, ctypes.c_int, ctypes.c_uint, ctypes.c_uint, ctypes.c_int, ctypes.c_int]
    x.XFlush.argtypes = [ctypes.c_void_p]; x.XCloseDisplay.argtypes = [ctypes.c_void_p]
    xtest.XTestFakeButtonEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_int, ctypes.c_ulong]
    x.XSetInputFocus(display, window, 1, 0); x.XWarpPointer(display, 0, window, 0, 0, 0, 0, x_pos, y_pos)
    x.XFlush(display); time.sleep(.15)
    xtest.XTestFakeButtonEvent(display, 1, 1, 0); x.XFlush(display); time.sleep(.1)
    xtest.XTestFakeButtonEvent(display, 1, 0, 0); x.XFlush(display); x.XCloseDisplay(display)

try:
    g.ready()
    for command in ('gamemode creative @s', 'clear @s', 'tp @s -0.5 71 0.5', 'setblock 0 71 2 minecraft:air'):
        g.call('command', command); g.call('wait', '8')
    g.call('open-inventory'); g.call('wait', '10')
    # Bottom-row column 5, at the standard 960x540 test resolution.
    screen_click(596, 420); g.call('wait', '10')
    g.call('screenshot', str(run / 'creative-blocks.png'))
    screen_click(319, 188); g.call('wait', '5')
    carried = g.call('inventory')['carried'] or {}
    assert carried.get('item') == 'tconstruct:amethyst_bronze_block', carried
    screen_click(319, 376); g.call('wait', '5'); g.call('close')
    assert any(s.get('item') == 'tconstruct:amethyst_bronze_block' for s in g.call('inventory')['hotbar'])
    g.call('place', '0', '71', '2', '--item', 'tconstruct:amethyst_bronze_block')
    assert g.call('block', '0', '71', '2')['block'] == 'tconstruct:amethyst_bronze_block'
    g.stop(); print('PASS Creative block group acquisition and placement', flush=True)
finally:
    g.cleanup()
