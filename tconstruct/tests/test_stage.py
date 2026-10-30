import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'tools/stage.py'

def git(repo, *args):
    return subprocess.check_output(['git', '-C', str(repo), *args], text=True).strip()

class StageExport(unittest.TestCase):
    def test_dirty_mixins_cli_source_is_not_built(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            repos = []
            for name in ('horizon', 'mixins'):
                repo = root / name
                repo.mkdir()
                git(repo, 'init', '-q')
                git(repo, 'config', 'user.name', 'Fixture')
                git(repo, 'config', 'user.email', 'fixture@example.invalid')
                (repo / 'marker').write_text('committed')
                git(repo, 'add', 'marker')
                git(repo, 'commit', '-qm', 'fixture')
                repos.append(repo)
            (repos[0] / 'marker').write_text('dirty horizon')
            (repos[1] / 'marker').write_text('dirty')
            project = root / 'project'
            (project / 'tools').mkdir(parents=True)
            (project / 'tests/fixtures').mkdir(parents=True)
            shutil.copy(SCRIPT, project / 'tools/stage.py')
            shutil.copy(SCRIPT.parent / 'portable_archive.py', project / 'tools/portable_archive.py')
            (project / 'compatibility.json').write_text(json.dumps(dict(horizon_revision=git(repos[0], 'rev-parse', 'HEAD'), mixins_revision=git(repos[1], 'rev-parse', 'HEAD'))))
            bindir = root / 'bin'
            bindir.mkdir()
            cargo = bindir / 'cargo'
            cargo.write_text('''#!/bin/sh
set -eu
[ "$(cat marker)" = committed ]
while [ "$1" != "--target-dir" ]; do shift; done
mkdir -p "$2/release"
printf '#!/bin/sh\\nexit 0\\n' > "$2/release/horizon-mixins"
chmod +x "$2/release/horizon-mixins"
''')
            cargo.chmod(0o755)
            env = dict(os.environ, PATH=str(bindir) + os.pathsep + os.environ['PATH'])
            subprocess.run([sys.executable, str(project / 'tools/stage.py'), '--horizon', str(repos[0]), '--mixins', str(repos[1]), '--mods', str(project / 'tests/fixtures')], env=env, check=True)
            self.assertEqual((repos[0] / 'marker').read_text(), 'dirty horizon')
            self.assertEqual((repos[1] / 'marker').read_text(), 'dirty')
            self.assertEqual((project / 'target/original/marker').read_text(), 'committed')
            self.assertEqual((project / 'target/mixins-source/marker').read_text(), 'committed')
            failed = subprocess.run([sys.executable, str(project / 'tools/stage.py'), '--horizon', str(repos[0]), '--mixins', str(repos[1]), '--mods', str(project / 'tests/fixtures')], env=env, capture_output=True)
            self.assertNotEqual(failed.returncode, 0)
            self.assertEqual((project / 'target/original/marker').read_text(), 'committed')
            custom = project / 'separate-output'
            subprocess.run([sys.executable, str(project / 'tools/stage.py'), '--horizon', str(repos[0]), '--mixins', str(repos[1]), '--mods', str(project / 'tests/fixtures'), '--output-dir', str(custom)], env=env, check=True)
            self.assertEqual((custom / 'original/marker').read_text(), 'committed')
