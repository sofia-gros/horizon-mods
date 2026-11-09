#!/usr/bin/env python3
"""Run a fresh loopback-only dedicated server and require exactly-once ordered shutdown events."""
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
    p.add_argument('--resources', type=Path, required=True)
    p.add_argument('--eula', type=Path, required=True, help='an existing owner-accepted eula.txt; never accepts an agreement itself')
    a = p.parse_args()
    agreed = [s.strip() for s in a.eula.read_text(encoding='utf-8').splitlines() if not s.strip().startswith('#')]
    if 'eula=true' not in agreed:
        p.error('--eula must name an already accepted eula.txt')
    runs = ROOT / '.local/smoke-runs'
    runs.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix='lifecycle-dedicated-', dir=runs)).resolve()
    import shutil
    shutil.copyfile(a.eula, run / 'eula.txt')
    (run / 'server.properties').write_text('server-ip=127.0.0.1\nserver-port=0\nonline-mode=false\nlevel-name=LifecycleProbe\nlevel-seed=42\nview-distance=2\nsimulation-distance=2\nspawn-chunk-radius=0\n', encoding='utf-8', newline='\n')
    env = dict(os.environ, HORIZON_NO_AUDIO='1', HORIZON_NO_DISCORD='1', HORIZON_TELEMETRY_URL='off', HORIZON_LAUNCHER_NO_SELF_UPDATE='1', RUST_MIN_STACK='16777216')
    for key in ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME'):
        folder = run / key.lower()
        folder.mkdir()
        env[key] = str(folder)
    log_path = run / 'game.log'
    with log_path.open('w', encoding='utf-8') as log:
        proc = subprocess.Popen([str(a.game.resolve()), '--resources', str(a.resources.resolve()), '--test-terrain', '--dedicated', '--nogui', '--port', '0', '--world', 'LifecycleProbe'], cwd=run, env=env, stdin=subprocess.PIPE, stdout=log, stderr=subprocess.STDOUT, text=True)
        try:
            deadline = time.monotonic() + 180
            while 'Done (' not in log_path.read_text(encoding='utf-8', errors='replace'):
                if proc.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError(f'dedicated server did not become ready: {log_path}')
                time.sleep(.25)
            proc.stdin.write('stop\n')
            proc.stdin.flush()
            proc.wait(timeout=90)
            assert proc.returncode == 0, proc.returncode
        finally:
            if proc.poll() is None:
                proc.terminate()
                try:
                    proc.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait(timeout=15)
    lines = log_path.read_text(encoding='utf-8', errors='replace').splitlines()
    events = [f'PLUTO-PROBE server_{phase} dedicated=true' for phase in ('starting', 'stopping', 'stopped')]
    assert all(lines.count(event) == 1 for event in events), (events, lines)
    assert [lines.index(event) for event in events] == sorted(lines.index(event) for event in events)
    assert 'Saving worlds' in '\n'.join(lines[:lines.index(events[-1])])
    print(json.dumps({'result': 'DEDICATED LIFECYCLE PASS', 'run': str(run), 'events': events}, indent=2))


if __name__ == '__main__':
    main()
