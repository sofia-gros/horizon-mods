"""Check that server mining wear is applied only to the acting player's stack."""
import argparse
import hashlib
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
    p.add_argument('--dedicated', action='store_true', help='Run a dedicated server and two clients')
    args = p.parse_args()
    for key in ('game', 'ctl', 'resources', 'client_mod', 'server_mod'):
        setattr(args, key, getattr(args, key).resolve())
    parent = ROOT / '.local/tconstruct-wear-tests'
    parent.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix='dedicated-players-' if args.dedicated else 'two-players-', dir=parent))
    report = {'ok': False, 'checks': [], 'game_sha256': hashlib.sha256(args.game.read_bytes()).hexdigest(),
              'server_mod_sha256': hashlib.sha256(args.server_mod.read_bytes()).hexdigest()}
    games = []

    def wait(label, predicate, timeout=20):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            value = predicate()
            if value: return value
            time.sleep(.2)
        raise AssertionError(label)

    def damage(game, amount):
        return wait(f'{game.label} state {amount} not observed',
                    lambda: (s if len(s := states(game.call('inventory'))) == 1 and s[0]['damage'] == amount else None))

    def check(label, condition):
        if not condition: raise AssertionError(label)
        report['checks'].append(label)

    def command(line):
        if args.dedicated:
            command_source.proc.stdin.write((line + '\n').encode())
            command_source.proc.stdin.flush()
            time.sleep(.4)
        else:
            command_source.call('command', line)

    def mine(game, x):
        command(f'setblock {x} 71 2 minecraft:stone')
        wait('block not delivered', lambda: game.call('block', str(x), '71', '2').get('block') == 'minecraft:stone')
        game.call('break', str(x), '71', '2', '--timeout', '30', timeout=35)
        game.call('wait', '3')

    try:
        print(f'RUN={run}', flush=True)
        if args.dedicated:
            server = Game(args, run, 'Server', dedicated=True)
            games.append(server)
            server.ready()
            host = Game(args, run, 'Host', server=f'127.0.0.1:{server.port}')
            games.append(host)
            host.ready()
            guest = Game(args, run, 'Guest', server=f'127.0.0.1:{server.port}')
            games.append(guest)
            guest.ready()
            command_source = server
            command('op Host')
            command('op Guest')
        else:
            host = Game(args, run, 'Host')
            games.append(host)
            host.ready()
            lan = host.call('lan', 'open', '0', 'commands')
            port = lan.get('port') or host.call('lan', 'status').get('port')
            if not port: raise RuntimeError('LAN port missing')
            guest = Game(args, run, 'Guest', server=f'127.0.0.1:{port}')
            games.append(guest)
            guest.ready()
            command_source = host
        for line in ('gamemode creative @a', 'clear @a', 'fill -2 70 -2 8 70 3 minecraft:stone', 'tp Host 0.5 71 0.5', 'tp Guest 5.5 71 0.5'):
            command(line)
        for game in (host, guest):
            game.call('chat', '/tconstruct specimen wood wood wood 0')
            damage(game, 0)
            game.call('select', '1')
            game.call('wait', '3')
            game.call('chat', '/tconstruct_tool activate')
            wait('activation failed', lambda: 'horizon:tool_control' in game.call('inventory')['held'].get('components', ''))
        command('gamemode survival @a')
        mine(host, 0)
        damage(host, 1)
        check('host break leaves guest tool unchanged', damage(guest, 0)[0]['damage'] == 0)
        mine(guest, 5)
        damage(guest, 1)
        check('guest break leaves host tool unchanged', damage(host, 1)[0]['damage'] == 1)
        mine(guest, 5)
        damage(guest, 2)
        check('consecutive guest breaks affect only guest', damage(host, 1)[0]['damage'] == 1)
        for game in reversed(games): game.stop()
        report['ok'] = True
    except Exception as error:
        report['error'] = f'{type(error).__name__}: {error}'
    finally:
        for game in reversed(games): game.cleanup()
        (run / 'report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
        print(json.dumps({'ok': report['ok'], 'report': str(run / 'report.json'), 'error': report.get('error')}), flush=True)
    return 0 if report['ok'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
