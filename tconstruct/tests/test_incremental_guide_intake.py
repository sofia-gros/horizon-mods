"""Source/inverse/resource checks only. Native/Rust tests are not invoked."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parents[2]
SOURCE=ROOT/'tconstruct/mixins/tconstruct'
RECEIPT=ROOT/'integration/guide-intake-20261107/incremental-modifiers.json'
def sha(raw):return hashlib.sha256(raw).hexdigest()
def blob(raw):return hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()
def inverse(path):
    row=json.loads(RECEIPT.read_bytes())['files'][path]
    text=(ROOT/path).read_text(encoding='utf-8')
    for old,new in reversed(row['replacements']):
        assert text.count(new)==1;text=text.replace(new,old,1)
    return text
def function(source,name):
    start=source.index('fn '+name+'(');start=source.index('{',start);depth=1;end=start+1
    while depth:
        depth+=(source[end]=='{')-(source[end]=='}');end+=1
    return source[start+1:end-1]

class IncrementalGuideIntake(unittest.TestCase):
    def test_twelve_changed_paths_inverse_to_exact_ordinary_parent(self):
        receipt=json.loads(RECEIPT.read_bytes())
        self.assertEqual(receipt['ordinary_base'],'7c4f846e3988e9d6494bc8b078728acfe98d27c2')
        self.assertEqual(receipt['source_head'],'c20813dcbdd1a1a18e60bbedc7d2741324b44e88')
        self.assertEqual(receipt['source_own_base'],'a17b0566acd81d584bfaec84caf8ca2558482c76')
        self.assertEqual(len(receipt['files']),12)
        for path,row in receipt['files'].items():
            with self.subTest(path=path):
                raw=(ROOT/path).read_bytes()
                self.assertEqual((sha(raw),blob(raw)),(row['after'],row['after_blob']))
                old=inverse(path).encode()
                self.assertEqual((sha(old),blob(old)),(row['before'],row['before_blob']))

    def test_station_limit_factoring_keeps_original_body_and_finite_decisions(self):
        path='tconstruct/mixins/tconstruct/source-upgrade-rules.rs'
        old=inverse(path);new=(ROOT/path).read_text(encoding='utf-8')
        first='''        // The dynamic snapshots encode fixed source increments. Reject a
        // changed denominator instead of consuming inputs for a wrong effect.
        if (id == "tconstruct:blasting" && needed != 20) || (id == "tconstruct:hydraulic" && needed != 36) { continue; }
'''
        a=function(old,'plan_with_traits');b=function(new,'plan_with_traits')
        self.assertEqual(a.count(first),1)
        a=a.replace(first,'',1).replace('if id == "tconstruct:hydraulic" && max != 5 { continue; }','if !incremental_limits_supported(id, needed, max) { continue; }',1)
        self.assertEqual(a,b)
        self.assertEqual(function(new,'incremental_limits_supported').strip(),'(id != "tconstruct:blasting" || needed == 20)\n        && (id != "tconstruct:hydraulic" || (needed == 36 && maximum == 5))')
        cases=0
        for name in ('tconstruct:blasting','tconstruct:hydraulic','tconstruct:haste','x:unknown'):
            for needed in (-1,0,1,20,35,36,45,2147483647):
                for maximum in (-1,0,1,4,5,6,2147483647):
                    prior=not((name=='tconstruct:blasting' and needed!=20) or (name=='tconstruct:hydraulic' and needed!=36)) and not(name=='tconstruct:hydraulic' and maximum!=5)
                    current=(name!='tconstruct:blasting' or needed==20) and (name!='tconstruct:hydraulic' or (needed==36 and maximum==5))
                    self.assertEqual(prior,current);cases+=1
        self.assertEqual(cases,224)
        for name in ('supported','tool_supported_with_tags','ordinary_modifier_supported','ingredient_matches','stack_ingredient_matches'):
            self.assertEqual(function(old,name),function(new,name),name)

    def test_incremental_projection_uses_existing_shared_owners_and_rejects_unknown_shapes(self):
        snapshot=(SOURCE/'modifier-snapshot.rs').read_text(encoding='utf-8')
        for token in ('pluto_api::resources::Layers','pluto_api::resources::visit_json_pack(',
                      'pluto_api::resources::highest(', 'tconstruct_source_tags::resolve_available_entries'):
            self.assertIn(token,snapshot)
        body=function(snapshot,'incremental_guide_recipes')
        for token in ('Some(1)','v == expected','v.len() == 1','needed.checked_mul(maximum).is_none()',
                      '!supported_limits(modifier, needed, maximum)','slots.len() != 1',
                      '"item" | "tag"','supported_tool(modifier, id) && matches(tool_filter, id)',
                      '!inputs.is_empty() && !tools.is_empty()','inputs.sort(); inputs.dedup(); tools.sort(); tools.dedup();'):
            self.assertIn(token,body)
        self.assertNotIn('"leftover"',body)
        self.assertIn('incremental_pages_refuse_unknown_shapes_denominators_and_unavailable_items',snapshot)

    def test_reader_is_puny_only_and_all_native_open_code_is_unchanged(self):
        path='tconstruct/mixins/tconstruct/source-materials-book.rs'
        old=inverse(path);new=(ROOT/path).read_text(encoding='utf-8')
        self.assertEqual(old[old.index('pub(super) fn open('):],new[new.index('pub(super) fn open('):])
        self.assertIn('if puny && key == "upgrades" { chapter_pages.extend(incremental_modifier_pages(game, &ordinary_tags)); }',new)
        for name in ('bonus_modifier_pages','ordinary_modifier_pages','dynamic_materials'):
            self.assertEqual(function(old,name),function(new,name))
        args=new.split('fn incremental_modifier_pages(',1)[1].split('{',1)[0]
        self.assertIn("game: &Game<'_>",args);self.assertNotIn('&mut',args)
        receipt=json.loads(RECEIPT.read_bytes())
        for name in ('rust','host','package','ui_lan'):self.assertEqual(receipt['validation'][name],'not run')

if __name__=='__main__':unittest.main()
