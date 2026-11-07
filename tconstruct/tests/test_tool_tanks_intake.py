"""Exact source/intake evidence, not a Rust, GUI or transfer execution."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]

def sha(raw): return hashlib.sha256(raw).hexdigest()
def blob(raw): return hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()

class ToolTanksIntake(unittest.TestCase):
    def test_nine_changed_files_inverse_to_declared_parent_or_source_pr(self):
        receipt = json.loads((ROOT/'integration/guide-intake-20261107/tool-tanks.json').read_bytes())
        self.assertEqual(receipt['ordinary_base'],'3a5a12e357564f744b460a033d512ce1ebc30a51')
        self.assertEqual(receipt['source_head'],'486995af9353a56598bf6217b9a381b3db3d321d')
        self.assertEqual(len(receipt['files']),9)
        for path,item in receipt['files'].items():
            with self.subTest(path=path):
                raw=(ROOT/path).read_bytes()
                incremental = json.loads((ROOT / 'integration/guide-intake-20261107/incremental-modifiers.json').read_bytes())
                self.assertEqual(incremental['ordinary_base'], '7c4f846e3988e9d6494bc8b078728acfe98d27c2')
                if path in incremental['files']:
                    item707 = incremental['files'][path]
                    self.assertEqual(sha(raw), item707['after'])
                    projected = raw.decode('utf-8')
                    for before707, after707 in reversed(item707['replacements']):
                        self.assertEqual(projected.count(after707), 1)
                        projected = projected.replace(after707, before707, 1)
                    raw = projected.encode()
                    self.assertEqual(sha(raw), item707['before'])

                ordinary = json.loads((ROOT / 'integration/guide-intake-20261107/ordinary-modifiers.json').read_bytes())
                self.assertEqual(ordinary['ordinary_base'], '33878a2167b3733135b680dea7990187c6cf7efb')
                if path in ordinary['files']:
                    item698 = ordinary['files'][path]
                    self.assertEqual(sha(raw), item698['after'])
                    projected = raw.decode('utf-8')
                    for before698, after698 in reversed(item698['replacements']):
                        self.assertEqual(projected.count(after698), 1)
                        projected = projected.replace(after698, before698, 1)
                    raw = projected.encode()
                    self.assertEqual(sha(raw), item698['before'])

                latest = json.loads((ROOT / 'integration/guide-intake-20261107/bonus-modifiers.json').read_bytes())
                self.assertEqual(latest['ordinary_base'], '5dde879335d73c69ab6a070ee5dccd90bcd3f89c')
                if path in latest['files']:
                    addition = latest['files'][path]
                    self.assertEqual(sha(raw), addition['after'])
                    projected = raw.decode('utf-8')
                    for old, new in reversed(addition['replacements']):
                        self.assertEqual(projected.count(new), 1)
                        projected = projected.replace(new, old, 1)
                    raw = projected.encode()
                    self.assertEqual(sha(raw), addition['before'])
                self.assertEqual(sha(raw),item['after']); self.assertEqual(blob(raw),item['after_blob'])
                text=raw.decode('utf-8')
                for old,new in reversed(item.get('replacements') or item.get('pr_adaptation',[])):
                    self.assertEqual(text.count(new),1); text=text.replace(new,old,1)
                if item['before']:
                    self.assertEqual(sha(text.encode()),item['before'])
                    self.assertEqual(blob(text.encode()),item['before_blob'])
                else: self.assertEqual(blob(text.encode()),item['pr_blob'])

    def test_five_implementation_inputs_remain_exact_current_sources(self):
        receipt = json.loads((ROOT/'integration/guide-intake-20261107/tool-tanks.json').read_bytes())
        self.assertEqual(len(receipt['unchanged_implementation_inputs']),5)
        for path,item in receipt['unchanged_implementation_inputs'].items():
            raw=(ROOT/path).read_bytes()
            self.assertEqual(sha(raw),item['sha256']); self.assertEqual(blob(raw),item['git_blob'])

    def test_documented_capacity_uses_the_existing_shared_loss_policy(self):
        shared=(ROOT/'pluto-lib/mixins/pluto_native_creative/stack-fluid.rs').read_text(encoding='utf-8')
        resize=shared.split('pub fn resize(',1)[1].split('#[cfg(test)]',1)[0]
        for token in ('read(stack, compound_key, fluid_key)?', 'if fluid.amount <= capacity',
                      'fluid.amount = capacity;', '(capacity > 0).then_some(&fluid)'):
            self.assertIn(token,resize)
        self.assertIn('let mut output = stack.clone();',shared)
        station=(ROOT/'tconstruct/mixins/tconstruct/source-station-world.rs').read_text(encoding='utf-8')
        self.assertIn('super::pluto_stack_fluid::resize(&probe',station)
        container=(ROOT/'tconstruct/mixins/tconstruct/source-fluid-container.rs').read_text(encoding='utf-8')
        self.assertIn('super::pluto_stack_fluid::read(input',container)
        self.assertIn('super::pluto_stack_fluid::write(input',container)

if __name__=='__main__': unittest.main()
