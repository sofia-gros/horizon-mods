"""Exercise TConstruct mining wear in an isolated single-player world."""
import argparse
import hashlib
import json
import math
import re
from pathlib import Path
import tempfile
import time

from probe_horizon_item_state import states
from probe_horizon_server_mods import Game, ROOT


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--game', type=Path, default=ROOT.parent / 'horizon/target/fast/horizon-client')
    parser.add_argument('--ctl', type=Path, default=ROOT.parent / 'horizon/target/fast/horizon-ctl')
    parser.add_argument('--resources', type=Path, default=ROOT.parent / 'horizon/.local/horizon-client/26.3')
    parser.add_argument('--client-mod', type=Path, default=ROOT / "Tinkers' onstruct/target/horizon-mod/tconstruct.hmod")
    parser.add_argument('--server-mod', type=Path, default=ROOT / "Tinkers' onstruct/server-tools/target/horizon-mod/tconstruct_tools.hmod")
    parser.add_argument('--in-frame', action='store_true')
    parser.add_argument('--lifecycle-probe', type=Path, help='Also load the server lifecycle probe and verify it responds')
    args = parser.parse_args()
    for key in ('game', 'ctl', 'resources', 'client_mod', 'server_mod'):
        setattr(args, key, getattr(args, key).resolve())
    args.extra_server_mods = [args.lifecycle_probe.resolve()] if args.lifecycle_probe else []
    parent = ROOT / '.local/tconstruct-wear-tests'
    parent.mkdir(parents=True, exist_ok=True)
    mode = 'in-frame' if args.in_frame else 'integrated'
    run = Path(tempfile.mkdtemp(prefix=mode + '-', dir=parent))
    report = {'ok': False, 'mode': mode, 'checks': [],
              'game_sha256': hashlib.sha256(args.game.read_bytes()).hexdigest(),
              'server_mod_sha256': hashlib.sha256(args.server_mod.read_bytes()).hexdigest()}
    game = None

    def wait(label, predicate, seconds=15):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            value = predicate()
            if value: return value
            time.sleep(.2)
        raise AssertionError(label)

    def check(label, predicate):
        if not predicate: raise AssertionError(label)
        report['checks'].append(label)

    def damage(expected):
        return wait(f'expected specimen damage {expected}',
                    lambda: (found if (found := sorted(s['damage'] for s in states(game.call('inventory')))) == expected else None))

    def projected_speed(expected):
        def read():
            components = game.call('inventory')['held'].get('components', '')
            match = re.search(r'(?<![a-z_])speed:([0-9.]+)f', components)
            return match and abs(float(match.group(1)) - expected) < 0.00001
        return wait(f'expected projected speed {expected}', read)

    def mine(block):
        game.call('command', f'setblock 0 71 2 minecraft:{block}')
        wait('block was not delivered', lambda: game.call('block', '0', '71', '2').get('block') == 'minecraft:' + block)
        result = game.call('break', '0', '71', '2', '--timeout', '30', timeout=35)
        game.call('wait', '3')
        return result

    def select(slot):
        game.call('select', str(slot))
        game.call('wait', '3')

    try:
        print(f'RUN={run}', flush=True)
        game = Game(args, run, 'Miner', in_frame=args.in_frame)
        game.ready()
        if args.lifecycle_probe:
            check('independent server mod responds', game.status().get('probe') == 'M2')
        for command in ('clear @s', 'fill -2 70 -2 2 70 3 minecraft:stone', 'tp @s 0.5 71 0.5'):
            game.call('command', command)
        game.call('chat', '/tconstruct specimen wood wood wood 58')
        game.call('chat', '/tconstruct specimen flint wood wood 0')
        damage([0, 58])
        for slot in (1, 2):
            select(slot)
            game.call('chat', '/tconstruct_tool activate')
            wait('profile not activated', lambda: 'horizon:tool_control' in game.call('inventory')['held'].get('components', ''))
        select(2)
        check('undamaged flint uses base speed', projected_speed(3.5))
        select(1)
        mine('stone')
        check('creative mining does not wear the tool', damage([0, 58]) == [0, 58])
        game.call('command', 'gamemode survival @s')
        mine('torch')
        check('instant break costs zero', damage([0, 58]) == [0, 58])
        mine('stone')
        check('ordinary mining costs one', damage([0, 59]) == [0, 59])
        mine('stone')
        check('durability limit enters broken state', damage([0, 60]) == [0, 60] and any(s['damage'] == 60 and s['broken'] for s in states(game.call('inventory'))))
        mine('dirt')
        check('broken tool remains and takes no further wear', damage([0, 60]) == [0, 60])
        select(2)
        mine('stone')
        mine('stone')
        check('switching tools isolates consecutive wear', damage([2, 60]) == [2, 60])
        check('jagged updates speed after wear', projected_speed(3.5 * (1 - 0.01 * math.sqrt(2))))
        game.call('command', 'gamemode creative @s')
        game.call('command', 'item replace entity @s hotbar.3 with minecraft:diamond_pickaxe')
        select(4)
        game.call('command', 'gamemode survival @s')
        mine('stone')
        inventory = game.call('inventory')
        report['vanilla_inventory'] = inventory
        check('vanilla tool remains and does not alter TConstruct states',
              damage([2, 60]) == [2, 60] and inventory['held'].get('item') == 'minecraft:diamond_pickaxe'
              and inventory['held'].get('count') == 1 and inventory['counts'].get('minecraft:wooden_pickaxe') == 2)
        if args.lifecycle_probe:
            check('independent server mod receives mining events', game.status().get('broken', 0) >= 5)
        game.call('save')
        game.stop()
        game = Game(args, run, 'Miner', in_frame=args.in_frame, resume=True)
        game.ready()
        if args.lifecycle_probe:
            check('independent server mod responds after restart', game.status().get('probe') == 'M2')
        inventory = game.call('inventory')
        check('both tool states survive restart without loss or duplication',
              damage([2, 60]) == [2, 60] and inventory['counts'].get('minecraft:wooden_pickaxe') == 2
              and inventory['counts'].get('minecraft:diamond_pickaxe') == 1)
        select(2)
        check('jagged speed survives restart', projected_speed(3.5 * (1 - 0.01 * math.sqrt(2))))
        mine('stone')
        check('wear continues after restart on the selected item', damage([3, 60]) == [3, 60])
        game.call('command', 'gamemode creative @s')
        game.call('chat', '/tconstruct_tool test-repair 3')
        check('repair restores base speed', damage([0, 60]) == [0, 60] and projected_speed(3.5))
        game.stop()
        report['ok'] = True
    except Exception as error:
        report['error'] = f'{type(error).__name__}: {error}'
    finally:
        if game: game.cleanup()
        (run / 'report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
        print(json.dumps({'ok': report['ok'], 'report': str(run / 'report.json'), 'error': report.get('error')}), flush=True)
    return 0 if report['ok'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
