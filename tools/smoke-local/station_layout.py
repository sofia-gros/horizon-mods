#!/usr/bin/env python3
"""Real-game gate for the station's dynamic slots; uses rendered UI coordinates and ordinary menu clicks.

Build with probe_pluto_lib.py --build-only first. Keeps screenshots and control responses in a fresh isolated run.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--game', type=Path, required=True)
    p.add_argument('--ctl', type=Path, required=True)
    p.add_argument('--resources', type=Path, required=True)
    p.add_argument('--runs', type=Path, default=ROOT / '.local/smoke-runs')
    a = p.parse_args()
    build = json.loads((a.game.resolve().parent / 'native-mods.json').read_text(encoding='utf-8'))
    compat = json.loads((ROOT / 'tconstruct/compatibility.json').read_text(encoding='utf-8'))
    assert build['horizon'] == compat['horizon_revision'] and build['horizon_mixins'] == compat['mixins_revision'], build
    assert not build['left_out'] and {'pluto_lib', 'pluto_probe', 'tconstruct'} <= {m['id'] for m in build['mods']}, build
    a.runs.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix='dynamic-station-', dir=a.runs)).resolve()
    endpoint = run / 'control.json'
    mod = run / 'horizon-mods/tconstruct'
    (mod / 'data/tconstruct/horizon/menu').mkdir(parents=True)
    (mod / 'horizon-mod.toml').write_text('id = "tconstruct"\nname = "Tinkers menus"\nversion = "0.5.0"\napi = "0.2"\nside = "both"\ncapabilities = ["registries"]\n', encoding='utf-8', newline='\n')
    import shutil
    shutil.copyfile(ROOT / 'tconstruct/content/data/tconstruct/horizon/menu/tinker_station.json', mod / 'data/tconstruct/horizon/menu/tinker_station.json')
    (mod / 'assets/tconstruct/textures/gui').mkdir(parents=True)
    shutil.copyfile(ROOT / 'tconstruct/assets/tconstruct/gui/tinker_station.png', mod / 'assets/tconstruct/textures/gui/tinker_station.png')
    (run / 'horizon-mods.txt').write_text('mod.tconstruct:true\n', encoding='utf-8', newline='\n')
    env = dict(os.environ, HORIZON_NO_AUDIO='1', HORIZON_NO_DISCORD='1', HORIZON_TELEMETRY_URL='off', HORIZON_LAUNCHER_NO_SELF_UPDATE='1', RUST_MIN_STACK='16777216')
    for key in ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME'):
        folder = run / key.lower()
        folder.mkdir()
        env[key] = str(folder)
    log = (run / 'game.log').open('w', encoding='utf-8')
    proc = subprocess.Popen([str(a.game.resolve()), '--resources', str(a.resources.resolve()), '--control-socket', str(endpoint), '--offscreen', '--world', 'DynamicStation', '--seed', '42', '--creative', '--test-terrain', '--render-distance', '2', '--width', '1280', '--height', '720'], cwd=run, env=env, stdout=log, stderr=subprocess.STDOUT)

    def ctl(*args, timeout=30):
        r = subprocess.run([str(a.ctl.resolve()), '--socket', str(endpoint), '--timeout', str(timeout), *map(str, args)], capture_output=True, text=True, timeout=timeout + 10)
        with (run / 'ctl.jsonl').open('a', encoding='utf-8', newline='\n') as f:
            f.write(json.dumps({'args': args, 'code': r.returncode, 'stdout': r.stdout, 'stderr': r.stderr}) + '\n')
        if r.returncode:
            raise RuntimeError((args, r.stdout, r.stderr))
        value = json.loads(r.stdout)
        if value.get('error'):
            raise RuntimeError(value)
        return value

    def ui_slots():
        value = ctl('ui')
        nodes = value.get('widgets', value.get('nodes', []))
        return {int(n['id'][5:]): n['bounds'] for n in nodes if n['id'].startswith('slot-')}

    def shot(name):
        ctl('screenshot', str(run / name))
        assert (run / name).is_file(), name
        print('SCREENSHOT', run / name, flush=True)

    try:
        deadline = time.monotonic() + 120
        while not endpoint.exists():
            if proc.poll() is not None or time.monotonic() > deadline:
                raise RuntimeError(f'game did not start: {run / "game.log"}')
            time.sleep(.2)
        ctl('wait-ready', '120', timeout=125)
        ctl('command', 'clear @s')
        pos = ctl('state')['player']['position']
        xyz = [int(pos[0] // 1) + 2, int(pos[1] // 1), int(pos[2] // 1)]
        ctl('command', f'setblock {xyz[0]} {xyz[1]} {xyz[2]} tconstruct:tinker_station')
        ctl('interact', *xyz)
        ctl('wait', '10')
        assert ctl('screen')['screen'] == 'container'
        slots = ui_slots()
        ox, oy = slots[304][0] - 114, slots[304][1] - 38

        def expect(expected):
            deadline = time.monotonic() + 10
            while True:
                slots = ui_slots()
                actual = {i - 300: [round(b[0] - ox, 2), round(b[1] - oy, 2)] for i, b in slots.items() if 300 <= i < 305}
                if actual == expected:
                    return
                if time.monotonic() > deadline:
                    raise AssertionError((expected, actual))
                time.sleep(.2)

        repair = {0: [53, 41], 1: [11, 41], 2: [31, 30], 3: [31, 50], 4: [114, 38]}
        expect(repair)
        shot('01-repair.png')
        # The independently loaded original data determines the selected button's index and expected positions.
        original = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/tinkering/station_layouts'
        layouts = [(path.stem, json.loads(path.read_text(encoding='utf-8'))) for path in original.glob('*.json')]
        layouts = [(name, data) for name, data in layouts if len(data['input_slots']) <= 3 and name != 'scorched_anvil']
        layouts.sort(key=lambda v: (v[0] != 'tinker_station', v[1].get('sortIndex', 0), v[0]))

        def select(name):
            i = next(i for i, (n, _) in enumerate(layouts) if n == name)
            ctl('click-at', ox - 116 + (i % 6) * 18 + 8, oy + 8 + (i // 6) * 18 + 8)
            ctl('wait', '5')

        ctl('command', 'give @s minecraft:quartz 1')
        ctl('click-slot', '0')
        ctl('click-at', ox + 61, oy + 49)
        ctl('wait', '5')
        select('pickaxe')
        pickaxe = {0: [112, 62], 1: [53, 22], 2: [15, 60], 3: [33, 42], 4: [114, 38]}
        expect(pickaxe)
        shot('02-pickaxe-occupied-dormant.png')
        # Click its new physical position, not its slot ID: item and hit testing must move together.
        ctl('click-at', ox + 120, oy + 70)
        ctl('wait', '5')
        carried = ctl('screen')['carried']
        assert carried and carried['item'] == 'minecraft:quartz', carried
        ctl('click-slot', '0')
        pickaxe.pop(0)
        expect(pickaxe)
        shot('03-pickaxe-empty-dormant.png')
        select('sword')
        expect({1: [48, 26], 2: [12, 62], 3: [30, 44], 4: [114, 38]})
        shot('04-sword.png')
        ctl('close')
        ctl('interact', *xyz)
        ctl('wait', '5')
        expect({1: [48, 26], 2: [12, 62], 3: [30, 44], 4: [114, 38]})
        select('tinker_station')
        expect(repair)
        shot('05-repair-restored.png')
        select('pickaxe')
        ctl('command', 'clear @s')
        for item, material, slot in [('pick_head', 'iron', 301), ('tool_handle', 'wood', 302), ('tool_binding', 'flint', 303)]:
            ctl('command', f'tconstruct part {item} {material}')
            ctl('wait', '5')
            inv = ctl('screen')['slots']
            source = next(s['slot'] for s in inv if s['slot'] < 36 and s.get('item') == f'tconstruct:{item}')
            ctl('click-slot', source)
            bounds = ui_slots()[slot]
            ctl('click-at', bounds[0] + 8, bounds[1] + 8)
            ctl('wait', '5')
        result = next(s for s in ctl('screen')['slots'] if s['slot'] == 304)
        assert result.get('item') == 'tconstruct:pickaxe', result
        assert 'tconstruct:iron' in result.get('components', ''), result
        shot('06-pickaxe-result.png')
        bounds = ui_slots()[304]
        ctl('click-at', bounds[0] + 8, bounds[1] + 8)
        ctl('wait', '5')
        result = ctl('screen')
        assert result['carried']['item'] == 'tconstruct:pickaxe', result
        assert all(s.get('item') is None for s in result['slots'] if 301 <= s['slot'] <= 304), result
        ctl('click-slot', '0')
        shot('07-crafted-inputs-consumed.png')
        ctl('close')
        ctl('command', f'setblock {xyz[0]} {xyz[1]} {xyz[2]} tconstruct:seared_melter')
        ctl('interact', *xyz)
        ctl('wait', '5')
        static = ui_slots()
        sx, sy = static[302][0] - 116, static[302][1] - 35
        assert [static[300][0] - sx, static[300][1] - sy] == [56, 17], static
        assert [static[301][0] - sx, static[301][1] - sy] == [56, 53], static
        shot('08-static-melter.png')
        ctl('close')
        ctl('quit')
        proc.wait(timeout=60)
        assert proc.returncode == 0, proc.returncode
        print(json.dumps({'result': 'STATION LAYOUT PASS', 'run': str(run), 'checks': ['original repair/pickaxe/sword coordinates', 'server button and menu data sync', 'occupied dormant slot moves and remains clickable', 'empty dormant slot disappears', 'close/reopen preserves selection', 'return to repair', 'build pickaxe from three material-bearing parts', 'take result consumes inputs exactly once', 'static melter unchanged', 'clean quit']}, indent=2), flush=True)
    finally:
        if proc.poll() is None:
            proc.terminate()
            try:
                proc.wait(timeout=15)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=15)
        log.close()
        print('RUN', run, flush=True)


if __name__ == '__main__':
    main()
