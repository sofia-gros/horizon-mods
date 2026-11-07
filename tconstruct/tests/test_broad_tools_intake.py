"""Source/inverse and finite policy oracle; does not execute Rust or a Game."""
import hashlib
import itertools
import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]
RECEIPT = ROOT / 'integration/guide-intake-20261107/broad-tools.json'
INDEX = ROOT / 'tconstruct/mixins/tconstruct/source-materials-book-index.rs'
TOOLS = ('sledge_hammer', 'vein_hammer', 'excavator', 'broad_axe', 'scythe', 'cleaver', 'longbow')

def sha(data): return hashlib.sha256(data).hexdigest()
def blob(data): return hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()

def inputs_ready(inputs, registered):
    # Independent JSON policy oracle, not the native tool_inputs implementation.
    if not inputs: return False
    for entry in inputs:
        part = entry.get('part')
        if isinstance(part, str):
            if part not in registered: return False
        else:
            choices = entry.get('choices')
            if not isinstance(choices, list) or not choices: return False
            if not all(isinstance(item, str) and item in registered for item in choices): return False
    return True

class BroadToolsIntake(unittest.TestCase):
    def test_all_own_delta_files_inverse_to_current_integration(self):
        receipt = json.loads(RECEIPT.read_bytes())
        self.assertEqual(receipt['ordinary_base'], '9e032ef5b013487ea08b997eb08df84825fb09ba')
        self.assertEqual(receipt['source_head'], '9567553edd0c932d9ceca32524ef44b008904771')
        self.assertEqual(len(receipt['files']), 14)
        for path, facts in receipt['files'].items():
            with self.subTest(path=path):
                raw = (ROOT / path).read_bytes()
                layer = json.loads((ROOT / 'integration/guide-intake-20261107/tool-tanks.json').read_bytes())
                self.assertEqual(layer['ordinary_base'], '3a5a12e357564f744b460a033d512ce1ebc30a51')
                if path in layer['files']:
                    later = layer['files'][path]
                    self.assertEqual(sha(raw), later['after'])
                    projected = raw.decode('utf-8')
                    for old, new in reversed(later['replacements']):
                        self.assertEqual(projected.count(new), 1)
                        projected = projected.replace(new, old, 1)
                    raw = projected.encode()
                    self.assertEqual(sha(raw), later['before'])
                self.assertEqual(sha(raw), facts['after'])
                self.assertEqual(blob(raw), facts['after_blob'])
                if facts['before'] is None:
                    self.assertEqual(blob(raw), facts['pr_blob'])
                else:
                    text = raw.decode('utf-8')
                    for old, new in reversed(facts['replacements']):
                        self.assertEqual(text.count(new), 1)
                        text = text.replace(new, old, 1)
                    self.assertEqual(sha(text.encode()), facts['before'])
                    self.assertEqual(blob(text.encode()), facts['before_blob'])
        for path, facts in receipt['unchanged_inputs'].items():
            raw = (ROOT / path).read_bytes()
            self.assertEqual(sha(raw), facts['sha256'])
            self.assertEqual(blob(raw), facts['git_blob'])

    def test_eligibility_rust_body_is_exact_bounded_conjunction(self):
        text = INDEX.read_text(encoding='utf-8')
        body = text.split('pub(super) fn mighty_tool_guide_ready(', 1)[1].split('\n}', 1)[0].split(' {\n', 1)[1]
        expected = ('registered && inputs_available && source_tool == Some(id) && matches!(id, '
                    + ' | '.join('"tconstruct:' + name + '"' for name in TOOLS) + ')')
        self.assertEqual(' '.join(body.split()), expected)

    def test_finite_tool_identity_registry_and_source_matrix(self):
        # Guarded by the exact Rust-body test. This finite oracle is not a Rust run.
        allowed = {'tconstruct:' + name for name in TOOLS}
        seen = accepted = 0
        for name, registered, inputs in itertools.product((*TOOLS, 'javelin', 'pickaxe', 'unknown'), (False, True), (False, True)):
            item = 'tconstruct:' + name
            for source in (item, 'other:' + name, '', None):
                result = registered and inputs and source == item and item in allowed
                expected = name in TOOLS and registered and inputs and source == item
                self.assertEqual(result, expected)
                seen += 1
                accepted += result
        self.assertEqual((seen, accepted), (160, 7))
        self.assertNotIn('other:longbow', allowed)

    def test_component_choices_require_every_displayed_registered_item(self):
        registered = {'part:head', 'part:handle', 'minecraft:stick'}
        self.assertTrue(inputs_ready([{'part':'part:head'}, {'choices':['part:handle','minecraft:stick']}], registered))
        for inputs in ([], [{}], [{'part':'missing'}], [{'choices':[]}], [{'choices':['part:head','missing']}],
                       [{'choices':['part:head', None]}], [{'choices':'part:head'}], [{'part':3}]):
            self.assertFalse(inputs_ready(inputs, registered), inputs)

    def test_current_source_admission_and_existing_mechanics_remain_in_place(self):
        reader = (ROOT / 'tconstruct/mixins/tconstruct/source-materials-book.rs').read_text(encoding='utf-8')
        start = reader.index('for load in list {')
        body = reader[start:reader.index('// The information group', start)]
        anchors = ['let Some(tags)', 'let Some(tools) = tags.get(tag)', 'tools.sort()', 'let Some(raw) = read(path, &file)',
                   'if mighty_advice', 'tools.partial.tool_notice', 'let inputs = tool_inputs(game, id)',
                   'mighty_tool_guide_ready', 'text.insert(0', 'let title = game.translate', 'chapter_pages.extend(tool_pages']
        locations = [body.index(value) for value in anchors]
        self.assertEqual(locations, sorted(locations))
        self.assertIn('BookId::from_item(book)?', reader)
        self.assertIn('ModifierAnchors::default', reader)
        self.assertIn('chapter_pages.splice(0..0, notices)', reader)
        self.assertIn('let lines = game.gui.atlas.font.split(&resolved, width);', reader)

if __name__ == '__main__': unittest.main()
