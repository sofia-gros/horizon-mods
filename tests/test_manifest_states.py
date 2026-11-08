import tomllib
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class ManifestStates(unittest.TestCase):
    def test_tinkers_keeps_one_state_because_mixins_plans_each_state_for_seconds(self):
        # Measured: about 7 s per [[state]] in `plan` and again in `apply`; 108 states cost 15 minutes, one state 29 s in all.
        # Add a field to the struct in tconstruct-state.rs instead of a new [[state]].
        manifest = tomllib.loads((ROOT / 'tconstruct/mixins/tconstruct/horizon-mixin.toml').read_text(encoding='utf-8'))
        self.assertEqual([state['id'] for state in manifest.get('state', [])], ['all-state'])


if __name__ == '__main__':
    unittest.main()
