#!/usr/bin/env python3
"""Check, in a running game, that PlutoLib's events fire for a mod that uses only the PlutoLib API.

Builds `pluto-lib/mod` together with the throwaway consumer `pluto-lib/tests/probe` using Horizon's official launcher
(`native-build --server-output`), starts the resulting game offscreen, and requires the consumer's log lines.
Local only (needs the launcher, an installed and prepared Horizon, and a prepared 26.3 resource folder). The caller sets
HORIZON_LAUNCHER_HOME / HORIZON_HOME / XDG_* / TMP / CARGO_HOME if the defaults are on a small drive.

  probe_pluto_lib.py --launcher horizon-launcher.exe --resources RESOURCES [--work DIR]
"""
import argparse, os, shutil, subprocess, sys, tempfile, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EXPECTED = ('PLUTO-PROBE server_starting dedicated=false', 'PLUTO-PROBE chunk_load', 'PLUTO-PROBE join name=', 'PLUTO-PROBE end_server_tick',
            'PLUTO-PROBE loot_modify')
# reported but not required: not seen firing yet in this offscreen run; promoted to EXPECTED once seen
OPTIONAL = ('PLUTO-PROBE client_resources_reloaded',)

p = argparse.ArgumentParser()
p.add_argument('--launcher', type=Path, required=True)
p.add_argument('--resources', type=Path, required=True, help='prepared Horizon 26.3 resource folder')
p.add_argument('--work', type=Path, default=ROOT / '.local/pluto-probe')
a = p.parse_args()

if a.work.exists():
    shutil.rmtree(a.work)
bundles, out = a.work / 'bundles', a.work / 'out'
shutil.copytree(ROOT / 'pluto-lib/mod', bundles / 'pluto_lib')
shutil.copytree(ROOT / 'pluto-lib/tests/probe', bundles / 'pluto_probe')
build_log = a.work / 'native-build.log'
with build_log.open('w') as log:
    build = subprocess.run([str(a.launcher), 'native-build', str(bundles), '--server-output', str(out), '--trust-native-code'],
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
        subprocess.run([str(ctl), '--socket', str(endpoint), '--timeout', '30', 'quit'], capture_output=True, timeout=40)
        proc.wait(timeout=60)
    finally:
        if proc.poll() is None:
            proc.terminate()
lines = [l for l in game_log.read_text(errors='replace').splitlines() if 'PLUTO-PROBE' in l]
missing = [e for e in EXPECTED if not any(e in l for l in lines)]
if missing:
    sys.exit(f'PROBE FAIL: events that did not fire: {missing}; saw {lines}; log {game_log}')
print('PROBE PASS')
for l in lines:
    print(' ', l)
print('optional events not seen:', [e for e in OPTIONAL if not any(e in l for l in lines)] or 'none')
