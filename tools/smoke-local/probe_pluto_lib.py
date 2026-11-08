#!/usr/bin/env python3
"""Check, in a running game, that PlutoLib's events fire for a mod that uses only the PlutoLib API.

Builds `pluto-lib/mod` together with the throwaway consumer `pluto-lib/tests/probe` using Horizon's official launcher
(`native-build --server-output`), starts the resulting game offscreen, and requires the consumer's log lines.
Local only (needs the launcher, an installed and prepared Horizon, and a prepared 26.3 resource folder). The caller sets
HORIZON_LAUNCHER_HOME / HORIZON_HOME / XDG_* / TMP / CARGO_HOME if the defaults are on a small drive.

  probe_pluto_lib.py --launcher horizon-launcher.exe --resources RESOURCES [--work DIR]
"""
import argparse, json, os, re, shutil, subprocess, sys, tempfile, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EXPECTED = ('PLUTO-PROBE server_starting dedicated=false', 'PLUTO-PROBE chunk_load', 'PLUTO-PROBE join name=', 'PLUTO-PROBE end_server_tick',
            'PLUTO-PROBE loot_modify', 'PLUTO-PROBE player send_message=true give_left=0',
            'PLUTO-PROBE attachment stored=true now=',
            'PLUTO-PROBE effects gave=true have=["minecraft:speed:1"]',
            'PLUTO-PROBE config_loading pluto_probe-common.toml text=',
            'tconstruct: gave the starting book to',
            'PLUTO-PROBE command ping arg=hello source=',
            'PLUTO-PROBE block_entity ticks=5 pos=[3, 80, 3] slots=3 water=500',
            'PLUTO-PROBE screen_draw origin_set=true',
            'PLUTO-PROBE renderer pos=',
            'PLUTO-PROBE fluids tconstruct=43',
            'PLUTO-PROBE punji east=Some("true")',
            'PLUTO-PROBE vine kept_on_stone=true',
            'PLUTO-PROBE crystal buds_found=true',
            'PLUTO-PROBE chime played=true',
            'PLUTO-PROBE seeds answer=Success block=Some("tconstruct:sky_earth_slime_grass")',
            'PLUTO-PROBE sapling grew=Some("tconstruct:greenheart_log")',
            'PLUTO-PROBE grass covered=Some("tconstruct:earth_slime_dirt")',
            'PLUTO-PROBE leaves near=Some("1") next=Some("2")',
            'PLUTO-PROBE vine after_support_removed=Some("minecraft:air")',
            'PLUTO-PROBE random_tick pos=[30, 81, 30] rolled_in_range=true set=true',
            'PLUTO-PROBE signs placed=[true, true, true, true]',
            'PLUTO-PROBE potted placed=[true, true]',
            'PLUTO-PROBE platform down=Some("true") up=Some("true")',
            'PLUTO-PROBE punji after_support_removed_later=Some("minecraft:air")',
            'PLUTO-PROBE world set=true read=Some("minecraft:glass")',
            'PLUTO-PROBE projectile kind=minecraft:snowball',
            'PLUTO-PROBE i18n language=en_us text=Hello from the probe ja=プローブからこんにちは player_language=',
            'pluto_lib: creative tabs added, tabs=',
            'PLUTO-PROBE item_use game_side=Some(Spec { food: Some((4, 0.5)), can_always_eat: false, remainder: Some(("minecraft:stick", 2)) })',
            'tconstruct: parts crafted for', 'tconstruct: pickaxe assembled for')
# reported but not required: not seen firing yet in this offscreen run; promoted to EXPECTED once seen
OPTIONAL = ('PLUTO-PROBE client_resources_reloaded',)

p = argparse.ArgumentParser()
p.add_argument('--launcher', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True, help='prepared Horizon 26.3 resource folder')
p.add_argument('--work', type=Path, default=ROOT / '.local/pluto-probe')
p.add_argument('--full', type=Path, metavar='PLUTO_INSTALLED',
               help='also build the whole sliced Tinkers (every injection the pinned Mixins accepts) with the legacy PlutoLib slice made from '
                    'this extracted legacy bundle folder (build.py leaves one at <work>/pluto-installed)')
a = p.parse_args()

if a.work.exists():
    shutil.rmtree(a.work)
bundles, out = a.work / 'bundles', a.work / 'out'
shutil.copytree(ROOT / 'pluto-lib/mod', bundles / 'pluto_lib')
shutil.copytree(ROOT / 'pluto-lib/tests/probe', bundles / 'pluto_probe')
# Tinkers' PlutoLib-only part: its manifest header (no injections, so Mixins has nothing to apply) and its api crate
tinkers = ROOT / 'tconstruct/mixins/tconstruct'
header = re.split(r'^\[\[', (tinkers / 'horizon-mixin.toml').read_text(encoding='utf-8'), maxsplit=1, flags=re.M)[0]
(bundles / 'tconstruct').mkdir()
(bundles / 'tconstruct/horizon-mixin.toml').write_text(header, encoding='utf-8', newline='\n')
shutil.copytree(tinkers / 'api', bundles / 'tconstruct/api')
if a.full:
    import json, tomllib
    config = json.loads((ROOT / 'tools/smoke-local/slice.json').read_text(encoding='utf-8'))
    shutil.rmtree(bundles / 'tconstruct')
    command = [sys.executable, str(ROOT / 'tools/slice_manifest.py'), '--tokens-out', str(a.work / 'tokens.txt'), '--out', str(bundles / 'tconstruct')]
    for ident in config['tconstruct_all_except']:
        command += ['--all-except', ident]
    for host_file in config.get('launcher_unsafe_files', []):
        command += ['--avoid-file', host_file]
    subprocess.run(command, check=True)
    legacy = a.full / 'pluto_native_creative/mixins/pluto_native_creative'
    names = {m['name'] for m in tomllib.loads((legacy / 'horizon-mixin.toml').read_text(encoding='utf-8'))['module']}
    command = [sys.executable, str(ROOT / 'tools/slice_manifest.py'), '--src', str(legacy), '--out', str(bundles / 'pluto_native_creative')]
    for seed in config['pluto_seeds']:
        command += ['--seed', seed]
    for prefix in config['pluto_keep_prefixes']:
        command += ['--keep-prefix', prefix]
    for name in sorted(names & set((a.work / 'tokens.txt').read_text().split())):
        command += ['--keep-module-name', name]
    subprocess.run(command, check=True)
build_log = a.work / 'native-build.log'
with build_log.open('w') as log:
    build = subprocess.run([str(a.launcher), 'native-build', str(bundles), '--server-output', str(out), '--trust-native-code', '--profile', 'dev'],
                           stdout=log, stderr=subprocess.STDOUT)
if build.returncode != 0 or not (out / 'horizon-engine.exe').is_file() and not (out / 'horizon-engine').is_file():
    sys.exit(f'PROBE FAIL (build): see {build_log}')
engine = next(x for x in (out / 'horizon-engine.exe', out / 'horizon-engine') if x.is_file())
ctl = next(x for x in (out / 'horizon-ctl.exe', out / 'horizon-ctl') if x.is_file())

run = Path(tempfile.mkdtemp(prefix='probe-', dir=a.work))
endpoint = run / 'control.json'
env = dict(os.environ, HORIZON_NO_AUDIO='1', HORIZON_NO_DISCORD='1', HORIZON_TELEMETRY_URL='off', RUST_MIN_STACK='16777216')
for key in ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME'):
    folder = run / key.lower(); folder.mkdir(); env[key] = str(folder)
game_log = run / 'game.log'
with game_log.open('w') as log:
    proc = subprocess.Popen([str(engine), '--resources', str(a.resources.resolve()), '--control-socket', str(endpoint), '--offscreen',
                             '--world', 'Probe', '--seed', '42', '--creative', '--test-terrain', '--render-distance', '2',
                             '--width', '640', '--height', '360'], cwd=run, env=env, stdout=log, stderr=subprocess.STDOUT)
    try:
        deadline = time.time() + 120
        while not endpoint.exists():
            if proc.poll() is not None or time.time() > deadline:
                sys.exit(f'PROBE FAIL (start): the game did not start; see {game_log}')
            time.sleep(.3)
        subprocess.run([str(ctl), '--socket', str(endpoint), '--timeout', '120', 'wait-ready', '120'], capture_output=True, timeout=130)
        time.sleep(3)
        subprocess.run([str(ctl), '--socket', str(endpoint), '--timeout', '30', 'command', 'pluto_probe ping hello'], capture_output=True, timeout=40)
        # a menu block two blocks from the player, to open its screen
        where = subprocess.run([str(ctl), '--socket', str(endpoint), '--timeout', '30', 'state'], capture_output=True, text=True, timeout=40).stdout
        try:
            position = json.loads(where)['player']['position']
            bx, by, bz = int(position[0] // 1) + 2, int(position[1] // 1), int(position[2] // 1)
        except (ValueError, KeyError, TypeError, IndexError):
            bx, by, bz = 3, 80, 3
        for line in ('setblock 3 80 3 tconstruct:seared_melter', f'setblock {bx} {by} {bz} tconstruct:seared_melter', 'give @s minecraft:snowball 1', 'look 0 90', 'use-item', 'summon minecraft:snowball ~ ~3 ~ {Motion:[0.0,-2.0,0.0]}', 'state', f'interact {bx} {by} {bz}', 'close', 'give @s minecraft:flint 1', 'give @s minecraft:stick 2', 'gamerule random_tick_speed 4000', 'tconstruct parts', 'tconstruct assemble', 'pluto_probe check'):
            # `interact X Y Z` is a control command (right-click a block); the rest are game commands
            words = ['interact', *line.split()[1:]] if line.startswith('interact ') else line.split() if line.startswith('look ') or line in ('use-item', 'close', 'state') else ['command', line]
            done = subprocess.run([str(ctl), '--socket', str(endpoint), '--timeout', '30', *words], capture_output=True, text=True, timeout=40)
            (run / 'ctl.log').open('a', encoding='utf-8').write(f'{words}: {done.returncode} {done.stdout.strip()} {done.stderr.strip()}\n')
            time.sleep(1)
        time.sleep(2)
        subprocess.run([str(ctl), '--socket', str(endpoint), '--timeout', '30', 'quit'], capture_output=True, timeout=40)
        proc.wait(timeout=60)
    finally:
        if proc.poll() is None:
            proc.terminate()
lines = [l for l in game_log.read_text(errors='replace').splitlines() if 'PLUTO-PROBE' in l or l.startswith(('tconstruct:', 'pluto_lib:'))]
missing = [e for e in EXPECTED if not any(e in l for l in lines)]
if missing:
    sys.exit(f'PROBE FAIL: events that did not fire: {missing}; saw {lines}; log {game_log}')
print('PROBE PASS')
for l in lines:
    print(' ', l)
print('optional events not seen:', [e for e in OPTIONAL if not any(e in l for l in lines)] or 'none')
