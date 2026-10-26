"""Exercise per-stack vanilla Tool settings through client prediction and server mining."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import tempfile

from probe_horizon_server_mods import Game, ROOT


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--game', type=Path, default=Path('A:/Project/horizon/target/fast/horizon-client.exe'))
    parser.add_argument('--ctl', type=Path, default=Path('A:/Project/horizon/target/fast/horizon-ctl.exe'))
    parser.add_argument('--resources', type=Path, default=Path(os.environ['APPDATA']) / 'horizon-launcher/versions/v0.2.66/.local/horizon-client/26.3')
    parser.add_argument('--in-frame', action='store_true')
    args = parser.parse_args()
    args.server_mod = None
    args.client_mod = None
    for key in ('game', 'ctl', 'resources'):
        setattr(args, key, getattr(args, key).resolve())
    parent = ROOT / '.local/tool-component-tests'
    parent.mkdir(parents=True, exist_ok=True)
    mode = 'in-frame' if args.in_frame else 'integrated'
    run = Path(tempfile.mkdtemp(prefix=mode + '-', dir=parent))
    report = dict(ok=False, mode=mode, checks=[], game_sha256=hashlib.sha256(args.game.read_bytes()).hexdigest())
    game = None

    def check(label, condition):
        if not condition:
            raise AssertionError(label)
        report['checks'].append(label)

    try:
        print(f'RUN={run}', flush=True)
        game = Game(args, run, 'Miner', in_frame=args.in_frame, mods=False)
        game.ready()
        for command in ('clear @s', 'gamerule doMobSpawning false', 'time set noon',
                        'fill -2 70 -2 2 70 3 minecraft:stone', 'tp @s 0.5 71 0.5', 'gamemode survival @s'):
            game.call('command', command)
        game.call('wait', '10')
        results = {}
        for label, speed, explicit in (('fast', 90.0, True), ('wood', 2.0, True), ('flint', 3.5, True), ('default', 0.5, False)):
            rules = f'{{blocks:"minecraft:stone",speed:{speed}f,correct_for_drops:true}}' if explicit else ''
            tool = f'{{rules:[{rules}],default_mining_speed:{speed}f,damage_per_block:1,can_destroy_blocks_in_creative:true}}'
            game.call('command', 'item replace entity @s hotbar.0 with minecraft:wooden_pickaxe[minecraft:tool=' + tool + ']')
            game.call('select', '1')
            game.call('wait', '3')
            inventory = game.call('inventory')
            check(label + ': Tool override reaches selected client stack', 'minecraft:tool' in inventory['held'].get('components', ''))
            game.call('command', 'setblock 0 71 2 minecraft:stone')
            result = game.call('break', '0', '71', '2', '--timeout', '30', timeout=40)
            results[label] = result
            check(label + ': block actually becomes air', 'minecraft:air' in json.dumps(game.call('block', '0', '71', '2')))
            check(label + ': harvest follows the stack rule',
                  result['drops'].get('minecraft:cobblestone', 0) == (1 if explicit else 0))
        (run / 'mining.json').write_text(json.dumps(results, indent=2), encoding='utf-8')
        ticks = {key: value['ticks'] for key, value in results.items()}
        report['mining_ticks'] = ticks
        check('fast override completes before prototype wooden mining could', ticks['fast'] <= 15)
        check('flint override is faster than wood on the same carrier', ticks['flint'] < ticks['wood'])
        check('empty rules use their overridden default speed', ticks['default'] > ticks['wood'] * 2)
        game.stop()
        game = Game(args, run, 'Miner', in_frame=args.in_frame, mods=False, resume=True)
        game.ready()
        check('Tool override survives save/reopen', 'default_mining_speed:0.5f' in game.call('inventory')['held'].get('components', ''))
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
