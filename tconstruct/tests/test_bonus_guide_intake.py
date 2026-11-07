"""Current-source/inverse checks for the partial bonus guide, not Rust execution."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parents[2]
SOURCE=ROOT/'tconstruct/mixins/tconstruct'
def sha(raw):return hashlib.sha256(raw).hexdigest()
def blob(raw):return hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()

class BonusGuideIntake(unittest.TestCase):
    def test_all_eight_modified_files_inverse_to_current_parent(self):
        receipt=json.loads((ROOT/'integration/guide-intake-20261107/bonus-modifiers.json').read_bytes())
        self.assertEqual(receipt['ordinary_base'],'5dde879335d73c69ab6a070ee5dccd90bcd3f89c')
        self.assertEqual(receipt['source_head'],'38a804e4543cde5172763469e36e313b50897e4b')
        self.assertEqual(len(receipt['files']),8)
        for path,item in receipt['files'].items():
            with self.subTest(path=path):
                raw=(ROOT/path).read_bytes();self.assertEqual(sha(raw),item['after']);self.assertEqual(blob(raw),item['after_blob'])
                text=raw.decode('utf-8')
                for old,new in reversed(item['replacements']):
                    self.assertEqual(text.count(new),1);text=text.replace(new,old,1)
                self.assertEqual(sha(text.encode()),item['before']);self.assertEqual(blob(text.encode()),item['before_blob'])

    def test_existing_pluto_resource_and_graph_owners_are_reused(self):
        snapshot=(SOURCE/'modifier-snapshot.rs').read_text(encoding='utf-8')
        for item in ('type Layers = pluto_api::resources::Layers;', 'pluto_api::resources::visit_json_pack(',
                     'pluto_api::resources::highest(', 'tconstruct_source_tags::resolve_available_entries(tag, &tags, supported)'):
            self.assertIn(item,snapshot)
        tags=(SOURCE/'source-tags.rs').read_text(encoding='utf-8')
        entry=tags.split('pub(super) fn resolve_available_entries(',1)[1].split('\n}',1)[0]
        self.assertEqual(entry.split(' {\n',1)[1].strip(),'resolve_available_blocks(id, tags, registry)')
        block=tags.split('pub(super) fn resolve_available_blocks(',1)[1].split('\n}',1)[0]
        self.assertIn('horizon_client::pluto_tag_graph::resolve(',block)
        self.assertIn('horizon_client::pluto_tag_graph::MissingValues::Ignore',block)
        self.assertNotIn('static ',entry)

    def test_partial_schema_and_omission_boundaries_remain_explicit(self):
        snapshot=(SOURCE/'modifier-snapshot.rs').read_text(encoding='utf-8')
        body=snapshot.split('pub(super) fn bonus_guide_recipes(',1)[1].split('/// Authoritative',1)[0]
        for item in ('v.len() == 1','Some("tconstruct:modifier")','Some(1)',
                     '"type" | "result" | "inputs" | "tools" | "level" | "allow_crystal"',
                     'inputs.sort(); inputs.dedup(); tools.sort(); tools.dedup();',
                     '!inputs.is_empty() && !tools.is_empty()'):
            self.assertIn(item,body)
        for name in ('supported_guide_projection_reloads_nested_tags_without_inventing_registry',
                     'bonus_guide_uses_current_recipes_items_and_eligible_tools'):
            self.assertIn('fn '+name+'()',snapshot)
        page=(SOURCE/'source-materials-book.rs').read_text(encoding='utf-8')
        self.assertIn('let mut modifier_anchors',page)
        self.assertIn('chapter_pages.extend(bonus_pages);',page)
        self.assertIn('Existing modifier and tool state still affect application.',page)
        docs=(ROOT/'tconstruct/docs/materials-and-you.md').read_text(encoding='utf-8')
        self.assertIn('a book already open is not refreshed in place',docs)
        self.assertIn('complete response size and LAN are unrun',docs)

if __name__=='__main__':unittest.main()
