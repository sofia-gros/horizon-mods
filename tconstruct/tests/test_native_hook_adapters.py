"""Keep native hook injection adapters thin while host hook tests scan their file."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'
ADAPTERS = {
    'source-casting-comparator-present': 'tconstruct_casting_comparator_present',
    'source-geode-random-tick': 'tconstruct_geode_random_tick',
    'source-sapling-random': 'tconstruct_sapling_random',
    'source-slime-leaves-random': 'tconstruct_slime_leaves_random',
    'source-slimy-grass-tick': 'tconstruct_slimy_grass_tick',
    'source-sapling-survive': 'tconstruct_sapling_survive',
    'source-overwield-harvest-start': 'tconstruct_overwield_harvest_start',
}

class NativeHookAdapters(unittest.TestCase):
    def test_adapters_delegate_without_macros(self):
        bodies = (ROOT / 'source-native-hook-bodies.rs').read_text()
        for file, name in ADAPTERS.items():
            adapter = (ROOT / (file + '.rs')).read_text()
            self.assertIn('super::tconstruct_native_hook_bodies::' + name + '(', adapter)
            self.assertIn('pub(super) fn ' + name + '(', bodies)
            code = '\n'.join(line for line in adapter.splitlines() if not line.lstrip().startswith('//'))
            for marker in ('!(', '![', '! {', '!{'):
                self.assertNotIn(marker, code, file)

if __name__ == '__main__':
    unittest.main()
