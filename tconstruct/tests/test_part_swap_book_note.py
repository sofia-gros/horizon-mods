"""Source text is retained while the partial casting implementation is disclosed."""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

class PartSwapBookNote(unittest.TestCase):
    def test_source_text_and_port_scope(self):
        path = ROOT / 'content/assets/tconstruct/book/materials_and_you/en_us/tools/part_swapping.json'
        page = json.loads(path.read_text())
        self.assertEqual(page['title'], 'Part Swapping')
        self.assertEqual(page['text'][0]['text'], 'Ever wish you could undo a past mistake? Luckily you can on tools. If you want to change the material of one of the parts, place the tool with the desired new part in the tinker station. For tools that have multiple of the same part, try a few different slots to replace the right one. For armor, you can often swap parts using a casting table.')
        self.assertEqual(page['text'][1]['paragraph'], True)
        self.assertIn('Armor casting swaps currently support only some cases', page['text'][1]['text'])
        self.assertIn('damaged equipment', page['text'][1]['text'])
        self.assertIn('modifiers', page['text'][1]['text'])
        doc = (ROOT / 'docs/materials-and-you.md').read_text()
        self.assertIn('Station part swapping is a separate path', doc)

if __name__ == '__main__':
    unittest.main()
