#!/usr/bin/env python3
"""Focused real-game gate for ServerPlayer.add_food; no menu or terrain preconditions."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--game', type=Path, required=True)
    parser.add_argument('--ctl', type=Path, required=True)
    parser.add_argument('--resources', type=Path, required=True)
    parser.add_argument('--server-reloads', type=int, default=0, help='also require one initial server-data event and one per /reload')
    parser.add_argument('--tooltip', action='store_true', help='also inspect and edit a real hovered rich tooltip, saving tooltip.png')
    args = parser.parse_args()
    if args.server_reloads < 0:
        parser.error('--server-reloads must be nonnegative')
    report = json.loads((args.game.resolve().parent / 'native-mods.json').read_text(encoding='utf-8'))
    # Explicit pins are checked below; this gate never accepts silently left-out mods.
    if report.get('horizon') != 'ebcfa2263aa3d8e399aa993b1e05daf42eb4cfa0' or report.get('horizon_mixins') != '5a64af938ec86b00d53d16a408aabc0038fb3e57':
        raise RuntimeError('food gate requires the reviewed Horizon/Mixins pins')
    if report.get('left_out') or not {'pluto_lib', 'pluto_probe', 'tconstruct'} <= {mod['id'] for mod in report['mods']}:
        raise RuntimeError('required native mods were not all built')
    runs = ROOT / '.local/smoke-runs'
    runs.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix='player-food-', dir=runs)).resolve()
    endpoint = run / 'control.json'
    env = dict(os.environ, HORIZON_NO_AUDIO='1', HORIZON_NO_DISCORD='1', HORIZON_TELEMETRY_URL='off', RUST_MIN_STACK='16777216')
    for key in ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME'):
        folder = run / key.lower()
        folder.mkdir()
        env[key] = str(folder)

    def ctl(*words, timeout=40):
        result = subprocess.run([str(args.ctl.resolve()), '--socket', str(endpoint), '--timeout', str(timeout), *words], capture_output=True, text=True, timeout=timeout + 5)
        with (run / 'ctl.log').open('a', encoding='utf-8') as output:
            output.write(f'{words}: {result.returncode} {result.stdout.strip()} {result.stderr.strip()}\n')
        if result.returncode:
            raise RuntimeError(f'control operation failed: {words}: {result.stdout} {result.stderr}')
        return json.loads(result.stdout)

    with (run / 'game.log').open('w', encoding='utf-8') as output:
        proc = subprocess.Popen([str(args.game.resolve()), '--resources', str(args.resources.resolve()), '--control-socket', str(endpoint), '--offscreen', '--world', 'FoodProbe', '--seed', '42', '--creative', '--test-terrain', '--render-distance', '2', '--width', '1280' if args.tooltip else '640', '--height', '720' if args.tooltip else '360'], cwd=run, env=env, stdout=output, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic() + 120
            while not endpoint.exists():
                if proc.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError(f'game did not start: {run / "game.log"}')
                time.sleep(.2)
            ctl('wait-ready', '120', timeout=125)
            ctl('command', 'pluto_probe feed')
            if args.server_reloads:
                for count in range(1, args.server_reloads + 2):
                    if count > 1:
                        ctl('command', 'reload', timeout=90)
                    marker = f'PLUTO-PROBE server_data_reloaded count={count}'
                    deadline = time.monotonic() + 60
                    while marker not in (run / 'game.log').read_text(encoding='utf-8', errors='replace').splitlines():
                        if proc.poll() is not None or time.monotonic() > deadline:
                            raise RuntimeError(f'missing server data notification {count}: {run / "game.log"}')
                        time.sleep(.1)
            if args.tooltip:
                ctl('command', 'clear @s')
                ctl('command', 'give @s minecraft:diamond[minecraft:custom_name={text:"Styled diamond",color:"gold",bold:true}] 1')
                ctl('select', 'minecraft:diamond')
                slot = ctl('state')['selected'] - 1
                ctl('command', 'gamemode survival')
                ctl('key', 'f3', 'down')
                ctl('key', 'h', 'tap')
                ctl('key', 'f3', 'up')
                assert ctl('status')['debug']['advanced_tooltips'], 'advanced tooltip mode did not turn on'
                ctl('open-inventory')
                ctl('wait', '10')
                ui = ctl('ui')
                nodes = ui.get('widgets', ui.get('nodes', []))
                bounds = next(node['bounds'] for node in nodes if node['id'] == f'slot-{slot}')
                x, y, width, height = bounds
                # Zero scroll steps only synthesize a CursorMoved event. Hover the
                # occupied slot without picking up the item or triggering double-clicks.
                point = (str(int(x + width / 2)), str(int(y + height / 2)))
                ctl('scroll-at', *point, '0')
                ctl('wait', '10')
                current = ctl('ui')
                slots = current.get('widgets', current.get('nodes', []))
                target = next(node for node in slots if node['id'] == f'slot-{slot}')
                assert target.get('stack'), ('hover unexpectedly moved the item', target)
                ctl('screenshot', str(run / 'tooltip.png'))
                ctl('close')
            state = ctl('state')
            assert state['player']['food'] == 14, state['player']
            ctl('screenshot', str(run / 'food.png'))
            ctl('quit')
            proc.wait(timeout=60)
            assert proc.returncode == 0, proc.returncode
        finally:
            if proc.poll() is None:
                proc.terminate()
                try:
                    proc.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait(timeout=15)
    lines = (run / 'game.log').read_text(encoding='utf-8', errors='replace').splitlines()
    feeding = [line for line in lines if line.startswith('PLUTO-PROBE feed ')]
    assert len(feeding) == 1 and 'feed pass=true ' in feeding[0], feeding
    lifecycle = [f'PLUTO-PROBE server_{phase} dedicated=false' for phase in ('starting', 'stopping', 'stopped')]
    assert all(lines.count(event) == 1 for event in lifecycle), lifecycle
    assert [lines.index(event) for event in lifecycle] == sorted(lines.index(event) for event in lifecycle)
    if args.server_reloads:
        reloaded = [line for line in lines if line.startswith('PLUTO-PROBE server_data_reloaded ')]
        expected = [f'PLUTO-PROBE server_data_reloaded count={count}' for count in range(1, args.server_reloads + 2)]
        assert reloaded == expected, (reloaded, expected)
        joined = next(index for index, line in enumerate(lines) if line.startswith('PLUTO-PROBE join name='))
        assert lines.index(expected[0]) < joined, 'initial data load must precede player join'
    if args.tooltip:
        tips = [line for line in lines if line.startswith('PLUTO-PROBE tooltip existing=')]
        assert len(tips) == 1 and 'existing="Styled diamond" removed_id=true preserved=true ' in tips[0], tips
        assert (run / 'tooltip.png').is_file()
    result = 'TOOLTIP EDIT PASS' if args.tooltip else 'SERVER DATA RELOAD PASS' if args.server_reloads else 'PLAYER FOOD PASS'
    print(json.dumps({'result': result, 'run': str(run), 'feeding': feeding[0], 'food': state['player']['food'], 'server_reloads': args.server_reloads, 'tooltip': args.tooltip}, indent=2))


if __name__ == '__main__':
    main()
