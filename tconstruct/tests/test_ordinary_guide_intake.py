"""Pinned source/resource/inverse checks; never launches Rust, Cargo or a game."""
import hashlib
import json
from pathlib import Path
import re
import unittest

ROOT=Path(__file__).resolve().parents[2]
SOURCE=ROOT/'tconstruct/mixins/tconstruct'
RECEIPT=ROOT/'integration/guide-intake-20261107/ordinary-modifiers.json'
def sha(raw):return hashlib.sha256(raw).hexdigest()
def blob(raw):return hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()
def previous_bytes(path):
    raw=(ROOT/path).read_bytes()
    receipt=json.loads((ROOT/'integration/guide-intake-20261107/incremental-modifiers.json').read_bytes())
    assert receipt['ordinary_base']=='7c4f846e3988e9d6494bc8b078728acfe98d27c2'
    if path in receipt['files']:
        entry=receipt['files'][path];assert sha(raw)==entry['after'];text=raw.decode('utf-8')
        for old,new in reversed(entry['replacements']):
            assert text.count(new)==1;text=text.replace(new,old,1)
        raw=text.encode();assert sha(raw)==entry['before']
    return raw

def invert(path):
    item=json.loads(RECEIPT.read_bytes())['files'][path]
    text=previous_bytes(path).decode('utf-8')
    for old,new in reversed(item['replacements']):
        assert text.count(new)==1
        text=text.replace(new,old,1)
    return text
def function(source,name):
    start=source.index('fn '+name+'(');start=source.index('{',start);depth=1;end=start+1
    while depth:
        depth+=(source[end]=='{')-(source[end]=='}');end+=1
    return source[start+1:end-1]

class OrdinaryGuideIntake(unittest.TestCase):
    def test_all_fifteen_paths_restore_current_parent_or_exact_source_resource(self):
        receipt=json.loads(RECEIPT.read_bytes())
        self.assertEqual(receipt['ordinary_base'],'33878a2167b3733135b680dea7990187c6cf7efb')
        self.assertEqual(receipt['source_head'],'a17b0566acd81d584bfaec84caf8ca2558482c76')
        self.assertEqual(receipt['source_own_base'],'38a804e4543cde5172763469e36e313b50897e4b')
        self.assertEqual(len(receipt['files']),15)
        for path,item in receipt['files'].items():
            with self.subTest(path=path):
                raw=previous_bytes(path)
                self.assertEqual((sha(raw),blob(raw)),(item['after'],item['after_blob']))
                if item['before'] is None:self.assertEqual(blob(raw),item['pr_blob'])
                else:
                    old=invert(path).encode()
                    self.assertEqual((sha(old),blob(old)),(item['before'],item['before_blob']))

    def test_station_refactor_preserves_complete_old_planner_and_predicate_bodies(self):
        path='tconstruct/mixins/tconstruct/source-upgrade-rules.rs'
        before=invert(path);after=previous_bytes(path).decode('utf-8')
        for name in ('supported','provisional_upgrade_enabled','ingredient_matches','stack_ingredient_matches'):
            self.assertEqual(function(before,name),function(after,name),name)
        old=function(before,'plan_with_traits');new=function(after,'plan_with_traits')
        expected_old='''        if !supported(id, &tool.item) {
            continue;
        }
        if id == "tconstruct:slurping" && (!tags.get("tconstruct:modifiable/interactable/charge").is_some_and(|items| items.contains(tool.item.as_ref()))
) { continue; }'''
        original_ids=re.search(r'if !matches!\(id, (.*?)\)\n\s*&& !BONUS_SLOTS',old).group(1)
        replaced=old.replace(expected_old,'        if !tool_supported_with_tags(id, &tool.item, tags) { continue; }')
        replaced=replaced.replace('if !matches!(id, '+original_ids+')\n                && !BONUS_SLOTS.iter().any(|(name, _, _)| *name == id) { continue; }','if !ordinary_modifier_supported(id) { continue; }')
        self.assertEqual(replaced,new)
        helper=function(after,'ordinary_modifier_supported')
        self.assertIn('matches!(id, '+original_ids+')',helper)
        self.assertIn('|| BONUS_SLOTS.iter().any(|(name, _, _)| *name == id)',helper)
        # Finite logical comparison of the factored gate, NOT Rust execution.
        ids=set(re.findall(r'"(tconstruct:[^"]+)"',before.split('const BONUS_SLOTS:',1)[1].split('];',1)[0]))
        ordinary=set(re.findall(r'"([^"]+)"',original_ids));new_ids=set(re.findall(r'"([^"]+)"',helper))
        cases=0
        for modifier in sorted(ids|ordinary|{'tconstruct:haste','x:unknown'}):
            for item_supported in (False,True):
                for charge_present in (False,True):
                    old_ok=item_supported and not (modifier=='tconstruct:slurping' and not charge_present) and (modifier in ordinary or modifier in ids)
                    new_ok=item_supported and (modifier!='tconstruct:slurping' or charge_present) and (modifier in new_ids or modifier in ids)
                    self.assertEqual(old_ok,new_ok);cases+=1
        self.assertEqual(cases,84)

    def test_separate_slots_and_conservative_schema_are_wired_to_real_consumers(self):
        snapshot=(SOURCE/'modifier-snapshot.rs').read_text(encoding='utf-8')
        body=function(snapshot,'ordinary_guide_recipes')
        for token in ('(1..=5).contains(&v.len())','inputs.iter().any(Vec::is_empty)',
                      'supported_tool(modifier, id) && matches(tool_filter, id)',
                      '"upgrades" | "abilities"','Some(1)','seen.insert(modifier)',
                      'i32::try_from(v).ok()','v.is_boolean()'):
            self.assertIn(token,body)
        reader=(SOURCE/'source-materials-book.rs').read_text(encoding='utf-8')
        page=function(reader,'ordinary_modifier_pages')
        self.assertIn('entry.inputs.iter().enumerate()',page)
        self.assertIn('entry.inputs.len() > 3',page)
        self.assertIn('tool_supported_with_tags(modifier, item, tags)',page)
        self.assertNotIn('&mut Game',reader.split('fn ordinary_modifier_pages(',1)[1].split('{',1)[0])
        old=invert('tconstruct/mixins/tconstruct/source-materials-book.rs')
        self.assertEqual(reader[reader.index('pub(super) fn open('):],old[old.index('pub(super) fn open('):])

    def test_existing_shared_owners_and_rust_fixture_dependency_identity_are_preserved(self):
        snapshot=(SOURCE/'modifier-snapshot.rs').read_text(encoding='utf-8')
        for name in ('pluto_api::resources::Layers','pluto_api::resources::highest(',
                     'pluto_api::resources::visit_json_pack(','tconstruct_source_tags::resolve_available_entries'):
            self.assertIn(name,snapshot)
        ranged=(ROOT/'tconstruct/tests/rust/ranged_upgrades.rs').read_text(encoding='utf-8')
        self.assertEqual(ranged.count('pub mod pluto_tag_graph;'),1)
        self.assertIn('api/src/resources.rs',ranged)
        self.assertIn('slurping_override_guide_and_station_share_resolved_charge_tag_gate',ranged)
        self.assertIn('ordinary_guide_preserves_five_slots_and_rejects_unknown_costs_and_shapes',snapshot)

    def test_retained_native_test_and_partial_claims_are_explicit(self):
        test=(ROOT/'tconstruct/tests/test_book_showcase_tags.py').read_text(encoding='utf-8')
        self.assertIn('test_actual_station_support_gates_exclude_incremental_and_disabled_ids',test)
        self.assertIn("subprocess.run(['rustc'",test)
        receipt=json.loads(RECEIPT.read_bytes())
        for field in ('rust','host','package','ui_lan'):self.assertEqual(receipt['validation'][field],'not run')
        docs=previous_bytes('tconstruct/docs/materials-and-you.md').decode('utf-8')
        for text in ('one to five ordinary input slots','Repeated ingredients retain separate slots',
                     'Incremental','held-page language refresh','No manifest'):
            self.assertIn(text,docs)

if __name__=='__main__':unittest.main()
