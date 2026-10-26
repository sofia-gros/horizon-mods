"""Exercise upstream server world API alongside the previously shipped TConstruct item API."""
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
    parser.add_argument('--game', type=Path, default=Path('A:/Project/horizon/target/fast/horizon-client.exe'))
    parser.add_argument('--ctl', type=Path, default=Path('A:/Project/horizon/target/fast/horizon-ctl.exe'))
    parser.add_argument('--resources', type=Path, default=Path(os.environ['APPDATA']) / 'horizon-launcher/versions/v0.2.66/.local/horizon-client/26.3')
    parser.add_argument('--client-mod', type=Path, default=ROOT / '.local/artifacts/m3b2-c0993fa5/tconstruct.hmod')
    parser.add_argument('--server-mod', type=Path, default=Path('A:/Project/horizon/target/mods-test/mods/beacons.hmod'))
    parser.add_argument('--old-server-mod', type=Path, default=ROOT / '.local/artifacts/m3b2-c0993fa5/tconstruct_tools.hmod')
    parser.add_argument('--in-frame', action='store_true')
    args = parser.parse_args()
    for key in ('game', 'ctl', 'resources', 'client_mod', 'server_mod', 'old_server_mod'):
        setattr(args, key, getattr(args, key).resolve())
    args.extra_server_mods = [args.old_server_mod]
    parent = ROOT / '.local/world-items-tests'
    parent.mkdir(parents=True, exist_ok=True)
    mode = 'in-frame' if args.in_frame else 'integrated'
    run = Path(tempfile.mkdtemp(prefix=mode + '-', dir=parent))
    report = dict(ok=False, mode=mode, checks=[], game_sha256=hashlib.sha256(args.game.read_bytes()).hexdigest(),
                  old_server_mod_sha256=hashlib.sha256(args.old_server_mod.read_bytes()).hexdigest())
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

    def inspect(expected_damage):
        request = uuid.uuid4().hex
        game.call('chat', '/tconstruct_tool inspect ' + request)
        def reply():
            for line in game.messages().get('chat', []):
                try:
                    obj = json.loads(line)
                except (ValueError, TypeError):
                    continue
                if isinstance(obj, dict) and obj.get('request') == request:
                    return obj
            return None
        value = wait_until('inspector did not reply', reply)
        check('old server package inspects the expected state', value['damage'] == expected_damage and value['parts'] == ['wood', 'wood', 'wood'])

    try:
        print(f'RUN={run}', flush=True)
        game = Game(args, run, 'WorldTester', in_frame=args.in_frame)
        game.ready()
        for command in ('clear @s', 'setblock 0 70 0 minecraft:stone', 'tp @s 0.5 71 0.5'):
            game.call('command', command)
        game.call('wait', '5')
        game.call('chat', '/ground')
        wait_until('world read did not report the ground', lambda: 'standing on minecraft:stone' in json.dumps(game.messages()))
        check('upstream world API reads the ground', True)
        game.call('chat', '/beacon')
        for y in range(71, 75):
            wait_until('world API did not place glowstone', lambda y=y: 'minecraft:glowstone' in json.dumps(game.call('block', '2', str(y), '0')))
        check('upstream world API applies all four blocks', True)
        game.call('chat', '/tconstruct specimen wood wood wood')
        wait_until('specimen was not created', lambda: len(states(game.call('inventory'))) == 1)
        game.call('select', '1')
        game.call('wait', '3')
        inspect(0)
        game.call('chat', '/tconstruct_tool test-damage 5')
        wait_until('old server package did not update the state', lambda: states(game.call('inventory'))[0]['damage'] == 5)
        check('old server package applies checked state edits beside world mod', True)
        inspect(5)
        game.stop()
        game = Game(args, run, 'WorldTester', in_frame=args.in_frame, resume=True)
        game.ready()
        check('world changes survive restart', 'minecraft:glowstone' in json.dumps(game.call('block', '2', '74', '0')))
        check('old package state survives restart', states(game.call('inventory'))[0]['damage'] == 5)
        inspect(5)
        game.call('command', 'setblock 0 70 0 minecraft:gold_block')
        game.call('chat', '/ground')
        wait_until('world read did not work after restart', lambda: 'standing on minecraft:gold_block' in json.dumps(game.messages()))
        check('world API remains usable after restart', True)
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
