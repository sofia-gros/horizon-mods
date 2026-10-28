"""Isolated Horizon game process for interactive integration tests."""
import json, os, subprocess, time
from pathlib import Path

class Game:
    def __init__(self, args, root, label, server=None, in_frame=False, resume=False):
        self.args, self.root, self.label = args, root / label, label
        self.root.mkdir(exist_ok=resume)
        self.endpoint = self.root / 'control.json'
        if resume: self.endpoint.unlink(missing_ok=True)
        self.env = dict(os.environ, HORIZON_NO_AUDIO='1', HORIZON_NO_DISCORD='1',
                        HORIZON_TELEMETRY_URL='off', RUST_MIN_STACK='16777216')
        for key in ('HORIZON_HOME','HORIZON_LAUNCHER_HOME','XDG_DATA_HOME','XDG_CONFIG_HOME'):
            folder = self.root / key.lower(); folder.mkdir(exist_ok=resume); self.env[key] = str(folder)
        width, height = os.environ.get('HORIZON_TEST_SIZE_' + label.upper(), os.environ.get('HORIZON_TEST_SIZE', '960x540')).split('x', 1)
        command = [str(args.game.resolve()), '--resources', str(args.resources.resolve()),
                   '--control-socket', str(self.endpoint), '--test-terrain', '--render-distance', '2',
                   '--offscreen', '--width', width, '--height', height, '--name', label]
        command += ['--server', server] if server else ['--world','Probe','--seed','42','--creative']
        if in_frame: command.append('--in-frame')
        self.log = (self.root / 'game.log').open('a' if resume else 'w')
        self.proc = subprocess.Popen(command, cwd=self.root, env=self.env, stdout=self.log, stderr=subprocess.STDOUT)
    def call(self, *words, timeout=30):
        p = subprocess.run([str(self.args.ctl.resolve()), '--socket', str(self.endpoint), '--timeout', str(timeout), *map(str,words)],
                           cwd=self.root, env=self.env, capture_output=True, text=True, timeout=timeout+5, check=True)
        result = json.loads(p.stdout)
        if result.get('error'): raise RuntimeError(str(result))
        return result
    def ready(self):
        deadline = time.monotonic()+90
        while not self.endpoint.exists():
            if self.proc.poll() is not None: raise RuntimeError(f'game exited; see {self.root / "game.log"}')
            if time.monotonic()>deadline: raise TimeoutError('control socket')
            time.sleep(.2)
        self.call('wait-ready','90',timeout=95)
    def stop(self):
        if self.proc.poll() is None: self.call('quit'); self.proc.wait(timeout=45)
        if self.proc.returncode != 0: raise RuntimeError(f'game exited {self.proc.returncode}')
    def cleanup(self):
        if self.proc.poll() is None:
            self.proc.terminate()
            try: self.proc.wait(timeout=10)
            except subprocess.TimeoutExpired: self.proc.kill(); self.proc.wait(timeout=10)
        self.log.close()
