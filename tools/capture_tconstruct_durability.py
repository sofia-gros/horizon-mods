"""Capture TConstruct's actual vanilla durability bar at three wear levels."""
import argparse
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
    args = parser.parse_args()
    for name in ('game', 'ctl', 'resources', 'client_mod', 'server_mod'):
        setattr(args, name, getattr(args, name).resolve())
    args.width, args.height = 1280, 720
    parent = ROOT / '.local/tconstruct-captures'
    parent.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix='durability-', dir=parent))
    report = {'ok': False, 'images': {}}
    game = None

    def wait(label, predicate):
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            result = predicate()
            if result:
                return result
            time.sleep(.2)
        raise AssertionError(label)

    def capture(label, expected):
        def state_ready():
            inv = game.call('inventory')
            found = states(inv)
            held = inv['held']
            return (len(found) == 1 and found[0]['damage'] == expected
                    and held.get('damage', 0) == expected
                    and '"minecraft:max_damage":85' in held.get('components', ''))
        wait('tool durability mismatch', state_ready)
        game.call('chat', f'/tconstruct_tool inspect {label}')
        wait('inspection missing', lambda: any(f'"request":"{label}"' in line for line in game.messages().get('chat', [])))
        game.call('wait', '8')
        path = run / f'{label}.png'
        game.call('screenshot', str(path), timeout=30)
        if not path.is_file() or path.read_bytes()[:8] != b'\x89PNG\r\n\x1a\n':
            raise AssertionError('screenshot failed')
        report['images'][label] = str(path)

    try:
        print(f'RUN={run}', flush=True)
        game = Game(args, run, 'Miner', in_frame=True)
        game.ready()
        game.call('command', 'clear @s')
        game.call('chat', '/tconstruct specimen flint wood wood 0')
        wait('specimen missing', lambda: len(states(game.call('inventory'))) == 1)
        game.call('select', '1')
        game.call('wait', '3')
        game.call('chat', '/tconstruct_tool activate')
        wait('profile not active', lambda: 'horizon:tool_control' in game.call('inventory')['held'].get('components', ''))
        capture('full', 0)
        game.call('chat', '/tconstruct_tool test-damage 42')
        capture('worn', 42)
        game.call('chat', '/tconstruct_tool test-damage 43')
        capture('broken', 85)
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
