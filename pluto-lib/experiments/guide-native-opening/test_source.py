"""Bounded source checks only; never builds/runs Rust or edits an official tree."""
import hashlib,json,re,unittest
from pathlib import Path
HERE=Path(__file__).resolve().parent
ROOT=HERE.parent.parent.parent
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
class Source(unittest.TestCase):
    def test_exact_group_and_original_fragments(self):
        meta=json.loads((HERE/'inputs.json').read_text())
        self.assertEqual({p.name for p in HERE.iterdir()},set(meta['files'])|{'inputs.json'})
        for name,digest in meta['files'].items(): self.assertEqual(sha(HERE/name),digest,name)
        for name,digest in meta['unchanged_fragments'].items(): self.assertEqual(sha(ROOT/name),digest,name)
    def test_one_allocator_and_native_assignment_not_replace(self):
        s=(HERE/'native-open-storage.rs').read_text()
        self.assertNotIn('static NEXT',s); self.assertNotIn('.replace(',s)
        order=['let Some(attachment)','let Some(opening)','slot.opening = None;','slot.revision = 0;','slot.value = Some(value);','slot.opening = Some(opening);','Ok(OwnedOpening']
        positions=[s.index(t) for t in order]; self.assertEqual(positions,sorted(positions))
        check=s[s.index('fn native_owner_is_current'):]
        for t in ['slot.value.is_some()','slot.attachment == Some(owner.attachment)','slot.opening == Some(owner.opening)']: self.assertIn(t,check)
        self.assertNotIn('revision',check)
    def test_exact_single_tail_and_post_effect_check(self):
        s=(HERE/'native-open-game.rs').read_text()
        tail='    game.keys.clear();\n    game.right = false;\n    game.capture(false);'
        self.assertEqual(s.count(tail),1)
        self.assertEqual(s.count('game.capture('),1); self.assertNotIn('.open_book(',s)
        self.assertLess(s.index('install_native_owned_value('),s.index(tail))
        self.assertLess(s.index(tail),s.index('if native_owner_is_current('))
        for t in ['Rejected {','Current(OwnedOpening)','Superseded','lectern: None','pages.is_empty()']: self.assertIn(t,s)
        for t in ['send(', 'lang::set(', 'source_pages(', 'history_mut(', 'request_sequence_mut(']: self.assertNotIn(t,s)
    def test_six_new_cases_and_no_mock_game(self):
        s=(HERE/'fixture.rs').read_text();meta=json.loads((HERE/'inputs.json').read_text())
        names=re.findall(r'#\[test\]\s*fn ([a-z0-9_]+)',s)
        self.assertEqual(names,meta['test_names']);self.assertEqual(len(names),6)
        self.assertNotIn('native-open-game.rs',s);self.assertNotIn('struct BookScreen',s)
    def test_excluded_from_ordinary_manifest(self):
        for name in ['pluto-lib/mixins/pluto_native_creative/horizon-mixin.toml','tconstruct/mixins/tconstruct/horizon-mixin.toml']:
            p=ROOT/name; self.assertTrue(p.is_file(),name)
            for token in ['guide-native-opening','native-open-game','native-open-storage']:
                self.assertNotIn(token,p.read_text())
if __name__=='__main__':unittest.main()
