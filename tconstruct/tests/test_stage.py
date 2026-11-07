import hashlib
import io
import json
import stat
import zipfile
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
            for name in ('portable_archive.py', 'stage_package_contract.py',
                         'prepare_playtest_bundles.py', 'playtest_pluto_inputs.py'):
                shutil.copy(SCRIPT.parent / name, project / 'tools' / name)
            guards = root / 'pluto-lib/tools'
            guards.mkdir(parents=True)
            (guards / 'check_native_plan.py').write_text("""import json, os, pathlib, sys
plan, workspace, contract_path = map(pathlib.Path, sys.argv[1:])
contract = json.loads(contract_path.read_bytes())
assert contract['mode'] == 'full-tconstruct'
assert {p['mod_id'] for p in contract['packages']} == {'tconstruct_native', 'pluto_native_creative'}
import hashlib
for package in contract['packages']:
    archive = pathlib.Path(package['archive'])
    assert archive.is_absolute()
    assert hashlib.sha256(archive.read_bytes()).hexdigest() == package['archive_sha256']
assert json.loads(plan.read_text()) == {'fixture_complete_plan': True}
assert (workspace / 'marker').read_text() == 'committed'
if os.environ.get('FIXTURE_REJECT_PREFLIGHT'):
    raise SystemExit(19)
(plan.parent / 'preflight-passed').write_text('passed')
""")
            revision = git(repos[1], 'rev-parse', 'HEAD')
            (project / 'compatibility.json').write_text(json.dumps(dict(horizon_revision=git(repos[0], 'rev-parse', 'HEAD'), mixins_repository=str(repos[1]), mixins_revision=revision)))
            horizon_revision = git(repos[0], 'rev-parse', 'HEAD')
            # These isolated Git repositories deliberately use fixture revisions.
            # Only the copied helpers' two revision constants change; production
            # helpers and all admission/contract functions remain unchanged.
            shared = SCRIPT.parents[2] / 'pluto-lib/tools/check_recipe_placement_plan.py'
            for original, copied in (
                (SCRIPT.parent / 'prepare_playtest_bundles.py', project / 'tools/prepare_playtest_bundles.py'),
                (shared, guards / 'check_recipe_placement_plan.py'),
            ):
                text = original.read_text(encoding='utf-8')
                for before, after in (
                    ('41f822b659789780f58d32c63ddfb6887ea1b8bf', horizon_revision),
                    ('5a64af938ec86b00d53d16a408aabc0038fb3e57', revision),
                ):
                    self.assertEqual(text.count(before), 1)
                    text = text.replace(before, after, 1)
                copied.write_text(text, encoding='utf-8')
            def archive(files):
                out = io.BytesIO()
                with zipfile.ZipFile(out, 'w') as z:
                    for name, value in sorted(files.items()):
                        info = zipfile.ZipInfo(name); info.create_system = 3
                        info.external_attr = (stat.S_IFREG | 0o644) << 16
                        z.writestr(info, value)
                return out.getvalue()
            mods = project / 'tests/fixtures'
            tc = mods / 'tconstruct_native'; (tc / 'data').mkdir(parents=True)
            (tc / 'horizon-mixin.toml').write_text(
                f'[mod]\nid="tconstruct_native"\nversion="0.5.0"\n[compatibility]\nhorizon_revision="{horizon_revision}"\n', encoding='utf-8')
            (tc / 'data/tconstruct-support.zip').write_bytes(archive({'tconstruct/content/fixture.txt': b'fixture'}))
            archives = root / 'archives'; archives.mkdir()
            def stage_args():
                values = []
                for identity, expanded, name, flag, version in (
                    ('tconstruct_native', tc, 'tconstruct.hnmod', 'tconstruct', '0.5.0'),
                    ('pluto_native_creative', pluto, 'plutolib.hnmod', 'pluto', '0.1.0'),
                ):
                    files = {'horizon-native.json': json.dumps({
                        'schema': 1, 'id': identity, 'version': version, 'game': '26.3',
                        'horizon': horizon_revision, 'mixins': revision}).encode('utf-8')}
                    files.update({'mixins/' + identity + '/' + p.relative_to(expanded).as_posix(): p.read_bytes()
                                  for p in expanded.rglob('*') if p.is_file()})
                    raw = archive(files); path = archives / name; path.write_bytes(raw)
                    values.extend(['--' + flag + '-archive', str(path),
                                   '--' + flag + '-archive-sha256', hashlib.sha256(raw).hexdigest()])
                return [sys.executable, str(project / 'tools/stage.py'), *values]
            pluto = root / 'installed/pluto_native_creative/mixins/pluto_native_creative'
            pluto.mkdir(parents=True)
            manifest = f'[mod]\nid = "pluto_native_creative"\nversion = "0.1.0"\n[compatibility]\nhorizon_revision = "{horizon_revision}"\n[[crate]]\nid = "api"\npath = "pluto_native_creative/api"\n'
            for name in ('pluto_transport_3a', 'pluto_tool_rules', 'pluto_component_display_3a', 'pluto_component_bar', 'pluto_items_b2'):
                manifest += f'[[module]]\nid = "{name}-fixture"\n'
            manifest += '[[crate]]\nid = "inventory"\npath = "pluto_native_creative/inventory-api"\n[[module]]\nid = "pluto_native_creative-inventory-export"\n'
            (pluto / 'horizon-mixin.toml').write_text(manifest)
            api = pluto / 'pluto_native_creative/api'
            api.mkdir(parents=True)
            (api / 'Cargo.toml').write_text('[package]\nname = "pluto-api"\nversion = "0.1.0"\n')
            inventory = pluto / 'pluto_native_creative/inventory-api'
            inventory.mkdir(parents=True)
            (inventory / 'Cargo.toml').write_text('[package]\nname = "pluto-inventory-api"\nversion = "0.1.0"\n')
            bindir = root / 'bin'
            bindir.mkdir()
            cargo = bindir / 'cargo'
            cargo.write_text('''#!/bin/sh
set -eu
case " $* " in *" --release "*) ;; *) exit 1 ;; esac
[ "$(cat marker)" = committed ]
while [ "$1" != "--target-dir" ]; do shift; done
mkdir -p "$2/release"
cat > "$2/release/horizon-mixins" <<'CLI'
#!/usr/bin/env python3
import json, pathlib, sys
out = pathlib.Path(sys.argv[sys.argv.index('--out') + 1])
if sys.argv[1] == 'plan':
    out.write_text(json.dumps({'fixture_complete_plan': True}))
elif sys.argv[1] == 'apply':
    assert (out.parent / 'preflight-passed').read_text() == 'passed'
    out.mkdir()
    (out / 'applied').write_text('applied')
else:
    raise SystemExit(2)
CLI
chmod +x "$2/release/horizon-mixins"
''')
            cargo.chmod(0o755)
            env = dict(os.environ, PATH=str(bindir) + os.pathsep + os.environ['PATH'])
            subprocess.run([*stage_args(), '--pluto-bundle', str(pluto), '--horizon', str(repos[0]), '--mixins', str(repos[1]), '--mods', str(project / 'tests/fixtures')], env=env, check=True)
            self.assertEqual((repos[0] / 'marker').read_text(), 'dirty horizon')
            self.assertEqual((repos[1] / 'marker').read_text(), 'dirty')
            self.assertEqual((project / 'target/original/marker').read_text(), 'committed')
            self.assertEqual((project / 'target/mixins-source/marker').read_text(), 'committed')
            self.assertEqual((project / 'target/mixins-revision.txt').read_text().strip(), revision)
            self.assertEqual((project / 'target/modded/applied').read_text(), 'applied')
            rejected_output = project / 'guard-rejected'
            rejected_guard = subprocess.run([*stage_args(), '--pluto-bundle', str(pluto), '--horizon', str(repos[0]), '--mixins', str(repos[1]), '--mods', str(project / 'tests/fixtures'), '--output-dir', str(rejected_output)], env=dict(env, FIXTURE_REJECT_PREFLIGHT='1'), capture_output=True)
            self.assertNotEqual(rejected_guard.returncode, 0)
            self.assertTrue((rejected_output / 'complete-plan.json').is_file())
            self.assertFalse((rejected_output / 'modded').exists())
            failed = subprocess.run([*stage_args(), '--pluto-bundle', str(pluto), '--horizon', str(repos[0]), '--mixins', str(repos[1]), '--mods', str(project / 'tests/fixtures')], env=env, capture_output=True)
            self.assertNotEqual(failed.returncode, 0)
            self.assertEqual((project / 'target/original/marker').read_text(), 'committed')
            custom = project / 'separate-output'
            subprocess.run([*stage_args(), '--pluto-bundle', str(pluto), '--horizon', str(repos[0]), '--mixins', str(repos[1]), '--mixins-revision', revision, '--mods', str(project / 'tests/fixtures'), '--output-dir', str(custom)], env=env, check=True)
            self.assertEqual((custom / 'original/marker').read_text(), 'committed')
            self.assertEqual((custom / 'mixins-revision.txt').read_text().strip(), revision)
            duplicate = project / 'tests/fixtures/pluto_tool_rules'
            duplicate.mkdir()
            rejected = subprocess.run([*stage_args(), '--pluto-bundle', str(pluto), '--horizon', str(repos[0]), '--mixins', str(repos[1]), '--mixins-revision', revision, '--mods', str(project / 'tests/fixtures'), '--output-dir', str(project / 'duplicate-output')], env=env, capture_output=True, text=True)
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn('duplicate or legacy', rejected.stderr)
            duplicate.rmdir()
            disguised = project / 'tests/fixtures/other-mod'
            (disguised / 'api').mkdir(parents=True)
            (disguised / 'horizon-mixin.toml').write_text('[[crate]]\nid = "api"\npath = "api"\n')
            (disguised / 'api/Cargo.toml').write_text('[package]\nname = "pluto-api"\nversion = "0.1.0"\n')
            rejected = subprocess.run([*stage_args(), '--pluto-bundle', str(pluto), '--horizon', str(repos[0]), '--mixins', str(repos[1]), '--mixins-revision', revision, '--mods', str(project / 'tests/fixtures'), '--output-dir', str(project / 'disguised-output')], env=env, capture_output=True, text=True)
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn('duplicate PlutoLib API crate', rejected.stderr)
            shutil.rmtree(disguised)
            old = api / 'Cargo.toml'
            old.write_text('[package]\nname = "pluto-creative-api"\nversion = "0.1.0"\n')
            rejected = subprocess.run([*stage_args(), '--pluto-bundle', str(pluto), '--horizon', str(repos[0]), '--mixins', str(repos[1]), '--mixins-revision', revision, '--mods', str(project / 'tests/fixtures'), '--output-dir', str(project / 'old-api-output')], env=env, capture_output=True, text=True)
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn('legacy PlutoLib API', rejected.stderr)
            old.write_text('[package]\nname = "pluto-api"\nversion = "0.1.0"\n')
            inventory_manifest = inventory / 'Cargo.toml'
            inventory_manifest.write_text('[package]\nname = "wrong-inventory"\nversion = "0.1.0"\n')
            rejected = subprocess.run([*stage_args(), '--pluto-bundle', str(pluto), '--horizon', str(repos[0]), '--mixins', str(repos[1]), '--mixins-revision', revision, '--mods', str(project / 'tests/fixtures'), '--output-dir', str(project / 'missing-inventory-output')], env=env, capture_output=True, text=True)
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn('inventory API', rejected.stderr)
            inventory_manifest.write_text('[package]\nname = "pluto-inventory-api"\nversion = "0.1.0"\n')
            duplicate_api = pluto / 'duplicate-api'
            duplicate_api.mkdir()
            (duplicate_api / 'Cargo.toml').write_text('[package]\nname = "pluto-api"\nversion = "0.1.0"\n')
            mixin_manifest = pluto / 'horizon-mixin.toml'
            mixin_manifest.write_text(manifest + '\n[[crate]]\nid = "duplicate"\npath = "duplicate-api"\n')
            rejected = subprocess.run([*stage_args(), '--pluto-bundle', str(pluto), '--horizon', str(repos[0]), '--mixins', str(repos[1]), '--mixins-revision', revision, '--mods', str(project / 'tests/fixtures'), '--output-dir', str(project / 'duplicate-crate-output')], env=env, capture_output=True, text=True)
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn('exactly one pluto-api', rejected.stderr)
            (duplicate_api / 'Cargo.toml').write_text('[package]\nname = "unrelated-helper"\nversion = "0.1.0"\n')
            allowed = project / 'unrelated-crate-output'
            subprocess.run([*stage_args(), '--pluto-bundle', str(pluto), '--horizon', str(repos[0]), '--mixins', str(repos[1]), '--mixins-revision', revision, '--mods', str(project / 'tests/fixtures'), '--output-dir', str(allowed)], env=env, check=True)
            mixin_manifest.write_text(manifest)
            (repos[1] / 'upstream-update').write_text('new revision')
            git(repos[1], 'add', 'upstream-update')
            git(repos[1], 'commit', '-qm', 'upstream update')
            latest = git(repos[1], 'rev-parse', 'HEAD')
            updated = project / 'latest-output'
            subprocess.run([*stage_args(), '--pluto-bundle', str(pluto), '--horizon', str(repos[0]), '--mixins', str(repos[1]), '--mods', str(project / 'tests/fixtures'), '--output-dir', str(updated)], env=env, check=True)
            self.assertNotEqual(revision, latest)
            self.assertEqual((updated / 'mixins-revision.txt').read_text().strip(), revision)
            self.assertFalse((updated / 'mixins-source/upstream-update').exists())
            explicit = project / 'explicit-newer-output'
            subprocess.run([*stage_args(), '--pluto-bundle', str(pluto), '--horizon', str(repos[0]), '--mixins', str(repos[1]), '--mixins-revision', latest, '--mods', str(project / 'tests/fixtures'), '--output-dir', str(explicit)], env=env, check=True)
            self.assertEqual((explicit / 'mixins-revision.txt').read_text().strip(), latest)
            self.assertEqual((explicit / 'mixins-source/upstream-update').read_text(), 'new revision')
            self.assertEqual((custom / 'mixins-revision.txt').read_text().strip(), revision)
