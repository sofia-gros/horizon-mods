"""Test initial TConstruct melee wear on direct mob hits in an isolated world."""
import argparse
import hashlib
import json
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
    args = parser.parse_args()
    for name in ('game', 'ctl', 'resources', 'client_mod', 'server_mod'):
        setattr(args, name, getattr(args, name).resolve())
    parent = ROOT / '.local/tconstruct-melee-tests'
    parent.mkdir(parents=True, exist_ok=True)
    mode = 'in-frame' if args.in_frame else 'integrated'
    run = Path(tempfile.mkdtemp(prefix=mode + '-', dir=parent))
    report = {'ok': False, 'mode': mode, 'checks': [],
              'game_sha256': hashlib.sha256(args.game.read_bytes()).hexdigest(),
              'server_mod_sha256': hashlib.sha256(args.server_mod.read_bytes()).hexdigest()}
    game = None

    def wait(label, predicate, timeout=20):
        until = time.monotonic() + timeout
        while time.monotonic() < until:
            result = predicate()
            if result:
                return result
            time.sleep(.2)
        raise AssertionError(label)

    def damage(expected):
        return wait(f'expected {expected}', lambda: (d if (d := sorted(s['damage'] for s in states(game.call('inventory')))) == expected else None))

    def check(label, condition):
        if not condition:
            raise AssertionError(label)
        report['checks'].append(label)

    def select(slot):
        game.call('select', str(slot))
        game.call('wait', '3')

    def attack():
        game.call('wait', '16')
        entities = game.call('entities', '8', 'iron_golem').get('entities', [])
        if not entities:
            raise AssertionError('test golem missing')
        result = game.call('attack', str(entities[0]['id']), timeout=30)
        if not result.get('hit') or result.get('health', 0) >= result.get('health_before', 0):
            raise AssertionError(f'attack did not land: {result}')
        report.setdefault('attacks', []).append(result)
        game.call('wait', '4')
        return result

    try:
        print(f'RUN={run}', flush=True)
        game = Game(args, run, 'Fighter', in_frame=args.in_frame)
        game.ready()
        for command in ('clear @s', 'fill -2 70 -2 2 70 4 minecraft:stone', 'tp @s 0.5 71 0.5',
                        'summon minecraft:iron_golem 0.5 71 2.5 {NoAI:1b}'):
            game.call('command', command)
        game.call('chat', '/tconstruct specimen wood wood wood 0')
        game.call('chat', '/tconstruct specimen wood wood wood 58')
        damage([0, 58])
        for slot in (1, 2):
            select(slot)
            game.call('chat', '/tconstruct_tool activate')
            wait('profile not activated', lambda: 'horizon:tool_control' in game.call('inventory')['held'].get('components', ''))
        select(1)
        attack()
        check('Creative hit does not consume core durability', damage([0, 58]) == [0, 58])
        game.call('command', 'gamemode survival @s')
        attack()
        check('survival mob hit costs two', damage([2, 58]) == [2, 58])
        select(2)
        attack()
        check('hit reaches broken threshold on the selected tool', damage([2, 60]) == [2, 60])
        check('broken tool remains', any(s['broken'] and s['damage'] == 60 for s in states(game.call('inventory'))))
        game.call('save')
        game.stop()
        game = Game(args, run, 'Fighter', in_frame=args.in_frame, resume=True)
        game.ready()
        check('both tool states survive restart', damage([2, 60]) == [2, 60])
        game.stop()
        report['ok'] = True
    except Exception as error:
        report['error'] = f'{type(error).__name__}: {error}'
    finally:
        if game:
            game.cleanup()
        (run / 'report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
        print(json.dumps({'ok': report['ok'], 'report': str(run / 'report.json'), 'error': report.get('error')}), flush=True)
    return 0 if report['ok'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
