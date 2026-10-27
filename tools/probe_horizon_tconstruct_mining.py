"""Exercise TConstruct material mining, broken/repair profiles, and persistence."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import tempfile
import time
import uuid

from probe_horizon_item_state import states
from probe_horizon_server_mods import Game, ROOT


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    if os.name == 'nt':
        game_dir = Path('A:/Project/horizon/target/fast')
        resource_dir = Path(os.environ.get('APPDATA', '')) / 'horizon-launcher/versions/v0.2.66/.local/horizon-client/26.3'
        suffix = '.exe'
    else:
        game_dir = ROOT.parent / 'horizon/target/fast'
        resource_dir = ROOT.parent / 'horizon/.local/horizon-client/26.3'
        suffix = ''
    parser.add_argument('--game', type=Path, default=game_dir / ('horizon-client' + suffix))
    parser.add_argument('--ctl', type=Path, default=game_dir / ('horizon-ctl' + suffix))
    parser.add_argument('--resources', type=Path, default=resource_dir)
    parser.add_argument('--client-mod', type=Path, default=ROOT / "Tinkers' onstruct/target/horizon-mod/tconstruct.hmod")
    parser.add_argument('--server-mod', type=Path, default=ROOT / "Tinkers' onstruct/server-tools/target/horizon-mod/tconstruct_tools.hmod")
    parser.add_argument('--in-frame', action='store_true')
    args = parser.parse_args()
    for key in ('game', 'ctl', 'resources', 'client_mod', 'server_mod'):
        setattr(args, key, getattr(args, key).resolve())
    parent = ROOT / '.local/tconstruct-mining-tests'
    parent.mkdir(parents=True, exist_ok=True)
    mode = 'in-frame' if args.in_frame else 'integrated'
    run = Path(tempfile.mkdtemp(prefix=mode + '-', dir=parent))
    report = dict(ok=False, mode=mode, checks=[], game_sha256=hashlib.sha256(args.game.read_bytes()).hexdigest(),
                  server_mod_sha256=hashlib.sha256(args.server_mod.read_bytes()).hexdigest())
    game = None

    def check(label, condition):
        if not condition:
            raise AssertionError(label)
        report['checks'].append(label)

    def wait_until(label, predicate):
        deadline = time.monotonic() + 12
        while True:
            value = predicate()
            if value:
                return value
            if time.monotonic() > deadline:
                raise AssertionError(label)
            time.sleep(0.2)

    def held():
        return game.call('inventory')['held'].get('components', '')

    def select(slot):
        game.call('select', str(slot))
        game.call('wait', '3')

    def inspect(damage, hook):
        request = uuid.uuid4().hex
        game.call('chat', '/tconstruct_tool inspect ' + request)
        def reply():
            for line in game.messages().get('chat', []):
                try:
                    value = json.loads(line)
                except (ValueError, TypeError):
                    continue
                if isinstance(value, dict) and value.get('request') == request:
                    return value
            return None
        value = wait_until('no inspector reply', reply)
        check('inspector reads core state and reports the mining hook', value['damage'] == damage and value['hooks'] == hook)

    def mine(block):
        game.call('command', f'setblock 0 71 2 minecraft:{block}')
        return game.call('break', '0', '71', '2', '--timeout', '30', timeout=35)

    try:
        print(f'RUN={run}', flush=True)
        game = Game(args, run, 'Miner', in_frame=args.in_frame)
        game.ready()
        for command in ('clear @s', 'fill -2 70 -2 2 70 3 minecraft:stone', 'tp @s 0.5 71 0.5'):
            game.call('command', command)
        for head, damage in (('wood', 0), ('flint', 17), ('flint', 85)):
            game.call('chat', f'/tconstruct specimen {head} wood wood {damage}')
        initial = wait_until('specimens did not arrive', lambda: (s if len(s := states(game.call('inventory'))) == 3 else None))
        check('three independent core states are present', sorted(x['damage'] for x in initial) == [0, 17, 85])
        for slot in (1, 2, 3):
            select(slot)
            game.call('chat', '/tconstruct_tool activate')
            wait_until('mining profile was not applied', lambda: 'horizon:tool_control' in held() and 'tconstruct_tools' in held())
        check('broken profile uses the original 0.3 speed', 'default_mining_speed:0.3f' in held())
        inspect(85, 'mining')
        game.call('command', 'gamemode survival @s')
        select(1)
        wood = mine('stone')
        wood_iron = mine('iron_ore')
        select(2)
        flint = mine('stone')
        flint_iron = mine('iron_ore')
        check('flint mines stone faster than wood', flint['ticks'] < wood['ticks'])
        check('wood cannot harvest iron ore but flint can', not wood_iron['drops'].get('minecraft:raw_iron', 0) and flint_iron['drops'].get('minecraft:raw_iron', 0) > 0)
        select(3)
        broken = mine('dirt')
        check('broken tool still mines slowly and remains in hand', broken['ticks'] >= 35 and 'minecraft:wooden_pickaxe' in json.dumps(game.call('inventory')['held']))
        check('game mining has not yet altered core wear', states(game.call('inventory')) == initial)
        game.call('command', 'gamemode creative @s')
        game.call('chat', '/tconstruct_tool test-repair 20')
        wait_until('repair did not update the selected state', lambda: any(x['damage'] == 65 and not x['broken'] for x in states(game.call('inventory'))))
        wait_until('repair did not restore the flint profile', lambda: 'speed:3.5f' in held())
        inspect(65, 'mining')
        game.call('command', 'gamemode survival @s')
        repaired = mine('stone')
        check('repaired tool regains flint mining speed', repaired['ticks'] < broken['ticks'])
        game.call('command', 'gamemode creative @s')
        game.call('chat', '/tconstruct_tool test-damage 20')
        wait_until('damage did not restore broken state', lambda: 'default_mining_speed:0.3f' in held())
        final_states = states(game.call('inventory'))
        check('re-breaking only changes the selected specimen', sorted(x['damage'] for x in final_states) == [0, 17, 85])
        report['ticks'] = dict(wood_stone=wood['ticks'], flint_stone=flint['ticks'], broken_dirt=broken['ticks'], repaired_stone=repaired['ticks'])
        game.stop()
        game = Game(args, run, 'Miner', in_frame=args.in_frame, resume=True)
        game.ready()
        check('three states survive save/reopen', states(game.call('inventory')) == final_states)
        check('broken profile survives save/reopen', 'default_mining_speed:0.3f' in held())
        inspect(85, 'mining')
        game.stop()
        report['ok'] = True
    except Exception as error:
        report['error'] = f'{type(error).__name__}: {error}'
    finally:
        if game:
            game.cleanup()
        (run / 'report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
        print(json.dumps(dict(ok=report['ok'], report=str(run / 'report.json'), error=report.get('error'))), flush=True)
    return 0 if report['ok'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
