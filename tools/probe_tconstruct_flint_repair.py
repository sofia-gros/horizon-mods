"""Exercise the experimental flint repair command in an isolated game."""
import argparse
import json
from pathlib import Path
import tempfile
import time

from probe_horizon_item_state import states
from probe_horizon_server_mods import Game, ROOT


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--game', type=Path, default=ROOT.parent / 'horizon/target/fast/horizon-client')
    p.add_argument('--ctl', type=Path, default=ROOT.parent / 'horizon/target/fast/horizon-ctl')
    p.add_argument('--resources', type=Path, default=ROOT.parent / 'horizon/.local/horizon-client/26.3')
    p.add_argument('--client-mod', type=Path, default=ROOT / "Tinkers' onstruct/target/horizon-mod/tconstruct.hmod")
    p.add_argument('--server-mod', type=Path, default=ROOT / "Tinkers' onstruct/server-tools/target/horizon-mod/tconstruct_tools.hmod")
    p.add_argument('--in-frame', action='store_true')
    args = p.parse_args()
    for key in ('game', 'ctl', 'resources', 'client_mod', 'server_mod'):
        setattr(args, key, getattr(args, key).resolve())
    parent = ROOT / '.local/repair-tests'
    parent.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix='repair-', dir=parent))
    report = {'ok': False, 'checks': [], 'mode': 'in-frame' if args.in_frame else 'integrated'}
    game = None

    def wait(label, predicate):
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            if predicate():
                report['checks'].append(label)
                return
            time.sleep(.2)
        raise AssertionError(label)

    def has(damage, flint):
        inv = game.call('inventory')
        return [s['damage'] for s in states(inv)] == [damage] and inv['counts'].get('minecraft:flint', 0) == flint

    try:
        game = Game(args, run, 'Miner', in_frame=args.in_frame)
        game.ready()
        game.call('command', 'clear @s')
        game.call('chat', '/tconstruct specimen flint wood wood 85')
        wait('broken specimen', lambda: has(85, 0))
        game.call('select', '1')
        game.call('chat', '/tconstruct_tool activate')
        wait('profile activated', lambda: 'horizon:tool_control' in game.call('inventory')['held'].get('components', ''))
        game.call('command', 'item replace entity @s hotbar.1 with minecraft:flint 3')
        wait('three flint', lambda: has(85, 3))
        game.call('command', 'gamemode survival @s')
        game.call('chat', '/tconstruct_tool repair-flint 1')
        wait('repair consumes two flint', lambda: has(0, 1))
        game.call('chat', '/tconstruct_tool repair-flint 1')
        game.call('wait', '10')
        wait('undamaged tool does not consume material', lambda: has(0, 1))
        game.call('save')
        game.stop()
        game = Game(args, run, 'Miner', in_frame=args.in_frame, resume=True)
        game.ready()
        wait('repair and remaining flint persist', lambda: has(0, 1))
        game.stop()
        report['ok'] = True
    except Exception as error:
        report['error'] = f'{type(error).__name__}: {error}'
    finally:
        if game:
            game.cleanup()
        (run / 'report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
        print(json.dumps({'report': str(run / 'report.json'), **report}), flush=True)
    return 0 if report['ok'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
