"""Exercise the generic atomic Tool profile API in isolated game worlds."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import tempfile
import time

from probe_horizon_server_mods import Game, ROOT


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--game', type=Path, default=Path('A:/Project/horizon/target/fast/horizon-client.exe'))
    parser.add_argument('--ctl', type=Path, default=Path('A:/Project/horizon/target/fast/horizon-ctl.exe'))
    parser.add_argument('--resources', type=Path, default=Path(os.environ['APPDATA']) / 'horizon-launcher/versions/v0.2.66/.local/horizon-client/26.3')
    parser.add_argument('--client-mod', type=Path, default=ROOT / "Tinkers' onstruct/target/horizon-mod/tconstruct.hmod")
    parser.add_argument('--server-mod', type=Path, default=Path('A:/Project/horizon/target/mods-test/mods/welcome.hmod'))
    parser.add_argument('--in-frame', action='store_true')
    args = parser.parse_args()
    for key in ('game', 'ctl', 'resources', 'client_mod', 'server_mod'):
        setattr(args, key, getattr(args, key).resolve())
    parent = ROOT / '.local/tool-profile-tests'
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

    def components():
        return game.call('inventory')['held'].get('components', '')

    def wait_for(label, predicate):
        until = time.monotonic() + 10
        while True:
            value = components()
            if predicate(value):
                return value
            if time.monotonic() > until:
                raise AssertionError(label + ': ' + value)
            time.sleep(0.2)

    def replace_and_break():
        game.call('command', 'setblock 0 71 2 minecraft:stone')
        result = game.call('break', '0', '71', '2', '--timeout', '15', timeout=20)
        check('configured Tool mines the stone quickly', result['ticks'] <= 15)

    try:
        print(f'RUN={run}', flush=True)
        game = Game(args, run, 'ProfileTester', in_frame=args.in_frame)
        game.ready()
        for command in ('clear @s', 'fill -2 70 -2 2 70 3 minecraft:stone', 'tp @s 0.5 71 0.5',
                        'item replace entity @s hotbar.0 with minecraft:wooden_pickaxe[custom_data={example:{state:"original"},foreign:9}]'):
            game.call('command', command)
        game.call('select', '1')
        game.call('wait', '3')
        game.call('chat', '/edit profile')
        initial = wait_for('disabled profile applied', lambda v: 'mining_enabled:0b' in v and 'changed' in v and 'minecraft:unbreakable' in v)
        check('atomic profile preserves unrelated custom data', 'foreign:9' in initial and 'minecraft:tool' in initial and 'welcome' in initial)
        game.call('chat', '/edit profile-unknown')
        deadline = time.monotonic() + 8
        while 'unknown tool block selector' not in json.dumps(game.messages()):
            if time.monotonic() > deadline:
                raise AssertionError('unknown block did not produce an application error')
            time.sleep(0.2)
        check('unknown block rejects the whole edit', components() == initial)
        game.stop()
        game = Game(args, run, 'ProfileTester', in_frame=args.in_frame, resume=True)
        game.ready()
        check('disabled profile survives save/reopen', 'mining_enabled:0b' in components())
        game.call('command', 'gamemode survival @s')
        game.call('command', 'setblock 0 71 2 minecraft:stone')
        try:
            game.call('break', '0', '71', '2', '--timeout', '2', timeout=8)
            blocked = False
        except RuntimeError:
            blocked = True
        check('disabled tool cannot mine in Survival', blocked and 'minecraft:stone' in json.dumps(game.call('block', '0', '71', '2')))
        game.call('chat', '/edit profile-enabled')
        wait_for('enabled profile applied', lambda v: 'mining_enabled:1b' in v and 'minecraft:unbreakable' in v)
        for _ in range(2):
            replace_and_break()
        check('unbreakable tool keeps its vanilla damage at zero', not re.search(r'"?minecraft:damage"?:\d', components()))
        game.call('chat', '/edit profile-zero')
        wait_for('zero-wear profile applied', lambda v: 'mining_enabled:1b' in v and 'minecraft:unbreakable' not in v and 'damage_per_block:0' in v)
        replace_and_break()
        check('zero damage-per-block keeps vanilla damage at zero', not re.search(r'"?minecraft:damage"?:\d', components()))
        game.call('chat', '/edit profile-wear')
        wait_for('wear profile applied', lambda v: 'mining_enabled:1b' in v and 'minecraft:unbreakable' not in v and 'damage_per_block:1' in v)
        replace_and_break()
        check('ordinary durability loss resumes when enabled', bool(re.search(r'"?minecraft:damage"?:1(?:[,}])', components())))
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
