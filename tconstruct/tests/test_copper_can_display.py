"""Validate the Copper Can RETURN injection's machine-consumed contract."""
from pathlib import Path
import tomllib
import unittest


class CopperCanDisplayContract(unittest.TestCase):
    def test_unit_return_hook_supplies_exactly_one_original_value(self):
        manifest = Path(__file__).resolve().parents[1] / 'mixins/tconstruct/horizon-mixin.toml'
        hooks = tomllib.loads(manifest.read_text())['injection']
        hook = next(item for item in hooks if item['id'] == 'copper-can-contents-gui')
        self.assertEqual(hook['kind'], 'RETURN')
        values = [binding for binding in hook['bindings'] if binding['value'] == '$value']
        self.assertEqual(len(values), 1)
        self.assertEqual(values[0]['ty'], '()')


if __name__ == '__main__':
    unittest.main()
