import importlib.util
import json
from pathlib import Path
import tempfile
import shutil
from unittest.mock import patch
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('validate_release', ROOT / 'tools/validate_release.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ReleaseValidationTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.path = Path(self.tmp.name)
        self.compat = json.loads((ROOT / 'tconstruct/compatibility.json').read_text())
        self.pluto = json.loads((ROOT / 'pluto-lib/horizon-native.json').read_text())
        self.native = dict(self.pluto, id='tconstruct_native', version='0.5.0')
        # The historical release-format validator needs a matching source fixture,
        # not the active empty product's version. This is not release readiness.
        checkout = self.path / 'checkout'
        for rel in ('tconstruct/compatibility.json', 'pluto-lib/horizon-native.json',
                    'pluto-lib/mod/horizon-mixin.toml',
                    'pluto-lib/tools/structure-boundaries.json',
                    'pluto-lib/tools/block-entity-boundaries.json'):
            target = checkout / rel
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes((ROOT / rel).read_bytes())
        source = checkout / 'tconstruct/mixins/tconstruct/horizon-mixin.toml'
        source.parent.mkdir(parents=True)
        source.write_text(self.mixin(dict(self.native, id='tconstruct')), encoding='utf-8', newline='\n')
        patcher = patch.object(module, 'ROOT', checkout)
        patcher.start()
        self.addCleanup(patcher.stop)
        self.write_packages()

    def archive(self, name, entries):
        with zipfile.ZipFile(self.path / name, 'w') as z:
            for key, value in entries.items():
                z.writestr(key, value)

    def mixin(self, manifest):
        return (f'[mod]\nid = "{manifest["id"]}"\nversion = "{manifest["version"]}"\n'
                f'[compatibility]\nhorizon_revision = "{manifest["horizon"]}"\n')

    def write_packages(self):
        self.archive('tconstruct.hmod', {'horizon-mod.toml': 'id = "tconstruct"\nversion = "0.5.0"\n',
                    'assets/tconstruct/a': 'a', 'data/tconstruct/a': 'a'})
        for filename, manifest in [('tconstruct.hnmod', self.native), ('plutolib.hnmod', self.pluto)]:
            self.archive(filename, {'horizon-native.json': json.dumps(manifest),
                         f'mixins/{manifest["id"]}/horizon-mixin.toml': self.mixin(manifest)})

    def test_correct_packages(self):
        module.validate_historical_tconstruct('0.5.0', self.path)

    def test_wrong_tag_fails(self):
        with self.assertRaises(ValueError):
            module.validate_historical_tconstruct('0.5.1', self.path)

    def test_mismatched_pins_fail(self):
        self.native['mixins'] = 'bad'
        self.write_packages()
        with self.assertRaises(ValueError):
            module.validate_historical_tconstruct('0.5.0', self.path)

    def test_wrong_pluto_metadata_fails(self):
        self.pluto['version'] = '9.0.0'
        self.write_packages()
        with self.assertRaises(ValueError):
            module.validate_historical_tconstruct('0.5.0', self.path)

    def test_pluto_lib_release(self):
        source = module.tomllib.loads((ROOT / 'pluto-lib/mod/horizon-mixin.toml').read_text())
        manifest = {'schema': 1, 'id': 'pluto_lib', 'version': source['mod']['version'], 'game': '26.3',
                    'horizon': self.compat['horizon_revision'], 'mixins': self.compat['mixins_revision']}
        self.archive('pluto-lib.hnmod', {'horizon-native.json': json.dumps(manifest),
                     'mixins/pluto_lib/horizon-mixin.toml': (ROOT / 'pluto-lib/mod/horizon-mixin.toml').read_text()})
        # The old metadata format remains testable, but is not installability evidence.
        module.validate_pluto_lib(manifest['version'], self.path)
        with self.assertRaises(ValueError):
            module.validate_pluto_lib('9.9.9', self.path)
        with self.assertRaisesRegex(ValueError, 'requires direct-host changes'):
            module.validate(f'pluto-lib-v{manifest["version"]}', self.path)
        with self.assertRaisesRegex(ValueError, 'requires direct-host changes'):
            module.preflight(f'pluto-lib-v{manifest["version"]}')

    def test_missing_or_malformed_distribution_contract_fails_closed(self):
        path = module.ROOT / 'pluto-lib/tools/structure-boundaries.json'
        for content in ('{', '{}', '[]', '{"direct_attachments": null}'):
            path.write_text(content, encoding='utf-8', newline='\n')
            with self.subTest(content=content), self.assertRaises(ValueError):
                module.require_supported_distribution()
        path.unlink()
        with self.assertRaisesRegex(ValueError, 'cannot establish'):
            module.require_supported_distribution()

    def test_either_contract_can_require_direct_host_distribution(self):
        path = module.ROOT / 'pluto-lib/tools/structure-boundaries.json'
        path.write_text('{"direct_attachments": []}', encoding='utf-8', newline='\n')
        with self.assertRaisesRegex(ValueError, 'requires direct-host changes'):
            module.require_supported_distribution()

    def test_untagged_product_is_refused(self):
        with self.assertRaises(ValueError):
            module.validate('v0.5.0', self.path)


class CleanReleaseValidationTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        for rel in ('docs/port-plan.json', 'tconstruct/compatibility.json'):
            target = self.root / rel
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(ROOT / rel, target)
        shutil.copytree(ROOT / module.port.BUNDLE, self.root / module.port.BUNDLE)
        patcher = patch.object(module, 'ROOT', self.root)
        patcher.start()
        self.addCleanup(patcher.stop)
        self.directory = self.root / '.local/dist'
        self.output = self.directory / 'tconstruct.hnmod'
        module.port.package(self.root, self.output)
        self.version = module.port.checked_files(self.root)[1]['mod']['version']

    def rewrite(self, mutate):
        with zipfile.ZipFile(self.output) as archive:
            entries = {name: archive.read(name) for name in archive.namelist()}
        mutate(entries)
        with zipfile.ZipFile(self.output, 'w') as archive:
            for name, payload in entries.items():
                archive.writestr(name, payload)

    def test_actual_clean_package_matches(self):
        module.validate_clean_tconstruct(self.version, self.directory)
        with self.assertRaisesRegex(ValueError, 'version'):
            module.validate_clean_tconstruct('999.0.0', self.directory)

    def test_changed_missing_and_extra_sources_reject(self):
        original = self.output.read_bytes()
        key = 'mixins/tconstruct_native/api/src/lib.rs'
        mutations = (
            lambda entries: entries.__setitem__(key, b'changed'),
            lambda entries: entries.pop(key),
            lambda entries: entries.__setitem__('mixins/tconstruct_native/extra.rs', b'extra'),
            lambda entries: entries.__setitem__('horizon-native.json', b'{}'),
        )
        for mutate in mutations:
            with self.subTest(mutation=mutate):
                self.output.write_bytes(original)
                self.rewrite(mutate)
                with self.assertRaisesRegex(ValueError, 'packaged inputs differ'):
                    module.validate_clean_tconstruct(self.version, self.directory)

    def test_retired_artifacts_reject(self):
        for name in ('tconstruct.hmod', 'plutolib.hnmod'):
            with self.subTest(name=name):
                artifact = self.directory / name
                artifact.write_bytes(b'old')
                with self.assertRaisesRegex(ValueError, 'retired'):
                    module.validate_clean_tconstruct(self.version, self.directory)
                artifact.unlink()

    def test_release_dispatch_refuses_unfinished_plan(self):
        plan_path = self.root / 'docs/port-plan.json'
        plan = json.loads(plan_path.read_text())
        # Deliberately unfinished fixture, independent of future roadmap completion.
        plan['tasks'] = [{'id': 'R5', 'kind': 'gameplay', 'status': 'pending', 'depends_on': []}]
        plan_path.write_text(json.dumps(plan), encoding='utf-8', newline='\n')
        with self.assertRaisesRegex(ValueError, 'not release-ready'):
            module.validate(f'tconstruct-v{self.version}', self.directory)


if __name__ == '__main__':
    unittest.main()
