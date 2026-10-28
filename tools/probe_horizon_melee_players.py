"""Check melee wear is applied only to the attacking player in LAN PvP."""
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
    parser.add_argument('--dedicated', action='store_true')
    args = parser.parse_args()
    for name in ('game', 'ctl', 'resources', 'client_mod', 'server_mod'):
        setattr(args, name, getattr(args, name).resolve())
    parent = ROOT / '.local/tconstruct-melee-tests'
    parent.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix='pvp-dedicated-' if args.dedicated else 'pvp-', dir=parent))
    report = {'ok': False, 'checks': [], 'game_sha256': hashlib.sha256(args.game.read_bytes()).hexdigest(),
              'server_mod_sha256': hashlib.sha256(args.server_mod.read_bytes()).hexdigest()}
    games = []

    def wait(label, predicate, timeout=20):
        until = time.monotonic() + timeout
        while time.monotonic() < until:
            result = predicate()
            if result:
                return result
            time.sleep(.2)
        raise AssertionError(label)

    def check(label, value):
        if not value:
            raise AssertionError(label)
        report['checks'].append(label)

    def damage(game, expected):
        return wait(f'{game.label} expected damage {expected}',
                    lambda: len(s := states(game.call('inventory'))) == 1 and s[0]['damage'] == expected)

    def command(line):
        if args.dedicated:
            server.proc.stdin.write((line + '\n').encode())
            server.proc.stdin.flush()
            time.sleep(.4)
        else:
            host.call('command', line)

    def position():
        command('tp Host 0.5 71 0.5')
        command('tp Guest 0.5 71 2.5')
        host.call('wait', '10')
        guest.call('wait', '10')

    def hit(attacker, target, x, z):
        position()
        attacker.call('look-at', str(x), '72', str(z))
        attacker.call('wait', '30')
        before = target.call('state')['player']['health']
        attacker.call('attack')
        attacker.call('wait', '10')
        after = target.call('state')['player']['health']
        report.setdefault('health', []).append([before, after])
        return after < before

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
            command('op Host')
            command('op Guest')
        else:
            host = Game(args, run, 'Host')
            games.append(host)
            host.ready()
            lan = host.call('lan', 'open', '0', 'commands')
            port = lan.get('port') or host.call('lan', 'status').get('port')
            if not port:
                raise RuntimeError('LAN port missing')
            guest = Game(args, run, 'Guest', server=f'127.0.0.1:{port}')
            games.append(guest)
            guest.ready()
        for line in ('gamemode creative @a', 'clear @a', 'fill -2 70 -2 2 70 4 minecraft:stone', 'gamerule pvp true'):
            command(line)
        for game in (host, guest):
            game.call('chat', '/tconstruct specimen wood wood wood 0')
            damage(game, 0)
            game.call('select', '1')
            game.call('wait', '3')
            game.call('chat', '/tconstruct_tool activate')
            wait('profile not active', lambda: 'horizon:tool_control' in game.call('inventory')['held'].get('components', ''))
        command('gamemode survival @a')
        check('host hit lands', hit(host, guest, .5, 2.5))
        check('host hit wears only host tool', damage(host, 2) and damage(guest, 0))
        check('guest hit lands', hit(guest, host, .5, .5))
        check('guest hit wears only guest tool', damage(guest, 2) and damage(host, 2))
        command('gamerule pvp false')
        check('disabled PvP rejects hit', not hit(host, guest, .5, 2.5))
        check('rejected hit consumes no durability', damage(host, 2) and damage(guest, 2))
        for game in reversed(games):
            game.stop()
        report['ok'] = True
    except Exception as error:
        report['error'] = f'{type(error).__name__}: {error}'
    finally:
        for game in reversed(games):
            game.cleanup()
        (run / 'report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
        print(json.dumps({'ok': report['ok'], 'report': str(run / 'report.json'), 'error': report.get('error')}), flush=True)
    return 0 if report['ok'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
