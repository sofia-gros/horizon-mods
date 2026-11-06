"""Guards for the negative capability probe; these do not run the planner."""
import importlib.util
import json
from pathlib import Path
import sys
import unittest
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT/'tools'))
spec = importlib.util.spec_from_file_location('probe_effect_state', ROOT/'tools/probe_effect_instance_state.py')
probe = importlib.util.module_from_spec(spec); spec.loader.exec_module(probe)

class ProbeContract(unittest.TestCase):
    def test_only_exact_rejection_is_accepted(self):
        valid = json.dumps({'code': 'E_STATE', 'message': probe.EXPECTED})
        self.assertTrue(probe.rejected(1, valid, False))
        for code, message in [('E_IO', probe.EXPECTED), ('E_STATE', 'constructor missing')]:
            self.assertFalse(probe.rejected(1, json.dumps({'code': code, 'message': message}), False))
        for status, text, output in [(0, valid, False), (-2, valid, False), (1, valid, True), (1, 'not json', False), (1, 'null', False), (1, '[]', False)]:
            self.assertFalse(probe.rejected(status, text, output))

    def test_probe_has_no_installed_runtime_entry(self):
        import tomllib
        manifest = tomllib.loads((ROOT/'tests/probes/effect-instance-state/horizon-mixin.toml').read_text())
        self.assertEqual(set(manifest), {'mod', 'compatibility', 'state'})
        self.assertEqual(len(manifest['state']), 1)
        state = manifest['state'][0]
        self.assertEqual(state['target'], 'horizon_client_render::effects::EffectInstance')
        self.assertEqual(state['ty'], 'Option<Vec<String>>')
        production = (ROOT/'mixins/pluto_native_creative/horizon-mixin.toml').read_text()
        self.assertNotIn('effect_state_probe', production)
        self.assertNotIn('owned-curatives', production)

if __name__ == '__main__': unittest.main()
