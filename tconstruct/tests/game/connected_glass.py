"""Exercise connected glass and panes in the reusable LAN test world."""
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
games = []


def wait(label, predicate):
    for _ in range(120):
        if predicate():
            return
        time.sleep(.2)
    raise AssertionError(label)


def command(game, text):
    game.call('command', text)
    game.call('wait', '8')


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
    host = Game(a, run, 'Player', in_frame=a.in_frame, resume=True); games.append(host); host.ready()
    port = host.call('lan', 'open', '0', 'commands')['port']
    guest = Game(a, run, 'Guest', server='127.0.0.1:' + str(port), resume=True); games.append(guest); guest.ready()
    for text in ('gamemode creative @a', 'kill @e[type=minecraft:zombie,tag=tconstruct_upgrade_test]',
                 'fill -2 70 -1 4 70 5 minecraft:stone', 'time set day', 'clear @a',
                 'fill -1 71 2 3 73 2 minecraft:air', 'tp @s -0.5 71 0.5', 'tp Guest 1.5 71 0.5'):
        command(host, text)
    # Select clear glass from the actual Creative tab before exercising placement.
    host.call('open-inventory'); host.call('wait', '10')
    screen_click(596, 420); host.call('wait', '10')
    nodes = host.call('ui')['widgets']
    glass = next((node for node in nodes if (node.get('stack') or {}).get('item') == 'tconstruct:clear_glass'), None)
    assert glass is not None, 'clear glass is absent from visible Creative slots'
    index = int(glass['id'].split('-')[1]) - 900
    screen_click(285 + (17 + (index % 9) * 18) * 2, 136 + (26 + (index // 9) * 18) * 2)
    host.call('wait', '5')
    assert (host.call('inventory')['carried'] or {}).get('item') == 'tconstruct:clear_glass'
    screen_click(319, 376); host.call('wait', '5'); host.call('close')
    host.call('place', '0', '71', '2', '--item', 'tconstruct:clear_glass')
    wait('Creative glass placed', lambda: host.call('block', '0', '71', '2').get('block') == 'tconstruct:clear_glass')
    # Two same blocks, another color, and a vertical continuation. The image
    # must be inspected against the original sprites after the test.
    for text in ('setblock 0 71 2 tconstruct:clear_glass', 'setblock 1 71 2 tconstruct:clear_glass',
                 'setblock 1 72 2 tconstruct:clear_glass', 'setblock 2 71 2 tconstruct:red_clear_stained_glass',
                 'setblock 3 71 2 tconstruct:seared_glass'):
        command(host, text)
    wait('guest sees colored glass', lambda: guest.call('block', '2', '71', '2').get('block') == 'tconstruct:red_clear_stained_glass')
    host.call('look-at', 'block', '1', '71', '2'); host.call('wait', '80')
    host.call('screenshot', str(run / 'connected-glass.png'))
    # Neighbors cross the x=15/16 chunk boundary; source panes use four state
    # properties and must update them on both host and guest.
    for text in ('fill 14 70 1 17 70 3 minecraft:stone', 'fill 15 71 2 16 72 2 minecraft:air',
                 'setblock 15 71 2 tconstruct:clear_glass_pane', 'setblock 16 71 2 tconstruct:clear_glass_pane',
                 'tp @s 14.5 71 0.5', 'tp Guest 16.5 71 0.5'):
        command(host, text)
    wait('pane connects east', lambda: 'east=true' in host.call('block', '15', '71', '2')['state'])
    wait('guest sees pane connection', lambda: 'west=true' in guest.call('block', '16', '71', '2')['state'])
    host.call('look-at', 'block', '15', '71', '2'); host.call('wait', '80')
    host.call('screenshot', str(run / 'connected-pane-boundary.png'))
    guest.stop(); host.call('save'); host.stop(); guest.cleanup(); host.cleanup(); games.clear()
    host = Game(a, run, 'Player', in_frame=a.in_frame, resume=True); games.append(host); host.ready()
    assert host.call('block', '15', '71', '2')['block'] == 'tconstruct:clear_glass_pane'
    assert 'east=true' in host.call('block', '15', '71', '2')['state']
    for text in ('gamemode survival @s', 'clear @s'):
        command(host, text)
    host.call('break', '15', '71', '2', '--timeout', '30', timeout=35)
    command(host, 'tp @s 15.5 71 2.5'); host.call('wait', '60')
    wait('pane self drop', lambda: any(s.get('item') == 'tconstruct:clear_glass_pane' for s in host.call('inventory')['hotbar']))
    host.stop()
    print('PASS connected glass LAN, pane state, save and self drop; inspect screenshots separately', flush=True)
finally:
    for game in reversed(games):
        game.cleanup()
