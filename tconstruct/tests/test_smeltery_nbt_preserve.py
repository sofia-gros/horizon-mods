from pathlib import Path
import unittest
ROOT = Path(__file__).resolve().parents[1]
class SmelteryMetadataWiring(unittest.TestCase):
    def test_both_host_targets_and_smeltery_only_are_wired(self):
        manifest = (ROOT / 'mixins/tconstruct/horizon-mixin.toml').read_text()
        store = (ROOT / 'mixins/tconstruct/source-melter-store.rs').read_text()
        self.assertEqual(manifest.count('source = "source-smeltery-nbt-preserve.rs"'), 2)
        self.assertIn('if preserve_smeltery && key == "tconstruct:smeltery"', store)
        self.assertIn('tconstruct_source_smeltery_nbt_preserve::merge_top(previous, &value)', store)
        self.assertIn('set_metadata(sources, *pos, key, value.clone())', store)
        receive = (ROOT / 'mixins/tconstruct/source-machine-wire-receive.rs').read_text()
        self.assertIn('tconstruct_source_melter_store::write_snapshot(', receive)
if __name__ == '__main__': unittest.main()
