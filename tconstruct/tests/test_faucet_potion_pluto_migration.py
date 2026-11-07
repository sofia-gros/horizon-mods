"""Source checks only: this suite does not compile Rust or emulate transactions."""
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
PROOF = ROOT / 'integration/pr700-pluto-intake'
RECEIPT = json.loads((PROOF / 'receipt.json').read_text())
SPEC = importlib.util.spec_from_file_location('pr700_intake', PROOF / 'apply.py')
INTAKE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(INTAKE)

def sha(data):
    return hashlib.sha256(data).hexdigest()

class SourceTests(unittest.TestCase):
    def test_all_four_workbranch_changes_reverse_exactly(self):
        for entry in RECEIPT['branch_changes']:
            data = (ROOT / entry['path']).read_bytes()
            self.assertEqual(sha(data), entry['after_sha256'])
            text = data.decode()
            for hunk in reversed(entry['hunks']):
                self.assertEqual(text.count(hunk['new']), 1)
                text = text.replace(hunk['new'], hunk['old'], 1)
            self.assertEqual(sha(text.encode()), entry['before_sha256'])

    def test_reuses_exact_catalog_module_without_new_policy(self):
        api = RECEIPT['new_shared_file']
        data = (ROOT / api['path']).read_bytes()
        self.assertEqual(sha(data), api['sha256'])
        self.assertIn('let def = potions::potion(id)?;', data.decode())
        manifest = tomllib.loads((ROOT / 'pluto-lib/mixins/pluto_native_creative/horizon-mixin.toml').read_text())
        entries = [m for m in manifest['module'] if m['id'] == 'potion-facts']
        self.assertEqual(entries, [{'id':'potion-facts','file':'crates/horizon-client/src/client_lib.rs','name':'pluto_potions','source':'potion-facts.rs','visibility':'pub'}])

    def test_consumer_keeps_tc_admission_and_native_lookup_behind_pluto(self):
        data = (ROOT / 'tconstruct/mixins/tconstruct/source-machine-runtime.rs').read_text()
        a=data.index('fn faucet_network_tag_safe(');b=data.index('fn faucet_supported_potion_tag(',a)
        code=data[a:b]
        for token in ['fields.len() == 1','Some("tconstruct:potion")','id.starts_with("minecraft:")','potion_tag(tag, false)','potion_fluid_name(Some(tag))','horizon_client::pluto_potions::potion(&id).is_some()']:
            self.assertIn(token,code)
        self.assertNotIn('horizon_client_core::potions::potion(',code)

    def test_route_cases_preserve_original_scope_and_add_catalog_parity(self):
        fixture=(ROOT / 'tconstruct/tests/rust/faucet_tank_casting_fixture.rs').read_text()
        names=re.findall(r'#\[test\]\s*fn\s+(\w+)',fixture)
        self.assertEqual(names,RECEIPT['native_route_tests'])
        self.assertEqual(len(names),12)
        for token in ['(160, 0, 90)','minecraft:strong_luck','CustomPotionColor','game.world.fail = true','assert_eq!(game.world.tags, before)','shared_catalog_is_the_native_catalog_without_policy_expansion']:
            self.assertIn(token,fixture)

    def test_actual_adapter_is_included_not_mocked(self):
        fixture=(ROOT / 'tconstruct/tests/rust/faucet_tank_casting_fixture.rs').read_text()
        runner=(ROOT / 'tconstruct/tests/run_faucet_tank_casting.py').read_text()
        self.assertIn('#[path="__POTION_FACTS__"] pub mod pluto_potions;',fixture)
        self.assertIn('template.replace("__POTION_FACTS__", str(root.parent / "pluto-lib/mixins/pluto_native_creative/potion-facts.rs"))',runner)
        self.assertIn('fn faucet_network_tag_safe(',runner)
        self.assertNotIn('mod pluto_potions {',fixture)

    def test_intake_is_only_four_current_paths_and_rejects_drift(self):
        document=json.loads((PROOF/'intake.json').read_text())
        self.assertEqual({c['path'] for c in document['changes']},INTAKE.ALLOWED)
        for c in document['changes']:
            with self.assertRaises(ValueError): INTAKE.transform(b'not-the-base',c)
        self.assertFalse(document['native_executed'])
        self.assertFalse(document['apply_to_main'])

    def test_transform_preserves_unaffected_bytes(self):
        data=b'alpha\nbeta\ngamma\n';out=b'alpha\nBETA\ngamma\n'
        c={'path':next(iter(INTAKE.ALLOWED)),'before_sha256':sha(data),'after_sha256':sha(out),'hunks':[{'start_line':1,'old':'beta\n','new':'BETA\n'}]}
        self.assertEqual(INTAKE.transform(data,c),out)
        c['hunks'][0]['old']='other\n'
        with self.assertRaises(ValueError): INTAKE.transform(data,c)

    def test_no_main_merge_or_native_success_claim(self):
        self.assertFalse(RECEIPT['main_merged'])
        self.assertFalse(RECEIPT['pr_closed'])
        self.assertFalse(RECEIPT['native_tests_run'])
        self.assertEqual(RECEIPT['status'],'changed-pending')
        self.assertEqual(RECEIPT['work_branch'],'feat/casting-tank-tagged-379')

if __name__ == '__main__': unittest.main()
