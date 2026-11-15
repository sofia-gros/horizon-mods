"""Tinkers host state belongs behind PlutoLib, never in its own manifest."""
import importlib.util
from pathlib import Path
import shutil
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('port_states', ROOT / 'tools/port.py')
port = importlib.util.module_from_spec(spec)
spec.loader.exec_module(port)


class ManifestStates(unittest.TestCase):
    def test_tinkers_state_injection_is_rejected_before_packaging(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            for rel in ('docs/port-plan.json', 'tconstruct/compatibility.json'):
                target = root / rel
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(ROOT / rel, target)
            shutil.copytree(ROOT / port.BUNDLE, root / port.BUNDLE)
            port.checked_files(root)
            manifest = root / port.BUNDLE / 'horizon-mixin.toml'
            with manifest.open('a', encoding='utf-8', newline='\n') as stream:
                stream.write('\n[[state]]\nid="all-state"\n')
            output = root / '.local/refused.hnmod'
            with self.assertRaisesRegex(ValueError, 'no injections/modules/state'):
                port.package(root, output)
            self.assertFalse(output.exists())


if __name__ == '__main__':
    unittest.main()
