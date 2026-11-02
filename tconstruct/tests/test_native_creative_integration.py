"""Run with TC_CREATIVE_RUNNER pointing to the shared API fixture runner."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/fixtures/creative-registration'


class NativeCreativeIntegration(unittest.TestCase):
    @unittest.skipUnless(os.environ.get('TC_CREATIVE_RUNNER'), 'shared API runner not supplied')
    def test_final_stack_reaches_tab_and_search_once(self):
        runner = os.environ['TC_CREATIVE_RUNNER']
        case = json.loads((FIXTURES / 'case.json').read_text())
        stack = (FIXTURES / 'finalized-stack.snbt').read_text().strip()
        case['groups'][0]['items'][1] = stack
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            input_path, output_path = root / 'input.json', root / 'output.json'
            input_path.write_text(json.dumps(case))
            subprocess.run([runner, str(input_path), str(output_path)], check=True)
            output = json.loads(output_path.read_text())
            tabs = {tab['id']: tab for tab in output['tabs']}
            self.assertEqual(tabs['tconstruct:tools']['items'], case['groups'][0]['items'])
            search = tabs['minecraft:search']
            self.assertEqual(search['items'].count(stack), 1)
            self.assertIn('tic_attack_damage', stack)
            self.assertIn('tic_materials', stack)
            self.assertEqual(search['lines'][0], ['Original stone tooltip'])
            self.assertEqual(search['tags'][0], ['minecraft:stone_tool_materials'])
            self.assertEqual(search['permissions_only'], [0])
            case['inventory'] = output
            input_path.write_text(json.dumps(case))
            second = subprocess.run([runner, str(input_path), str(root / 'duplicate.json')], capture_output=True)
            self.assertNotEqual(second.returncode, 0)
            self.assertFalse((root / 'duplicate.json').exists())


if __name__ == '__main__':
    unittest.main()
