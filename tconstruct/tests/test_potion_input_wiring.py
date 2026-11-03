"""Keep source container-slot callers supplied with the bottle tag snapshot."""
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1] / 'mixins/tconstruct'


class PotionInputWiring(unittest.TestCase):
    def test_all_smeltery_slot_transfer_paths_forward_bottle_tags(self):
        counts = {
            'source-smeltery-click.rs': ('transfer_selected_with_color(', 3),
            'source-smeltery-pointer.rs': ('transfer_selected_with_color(', 2),
            'source-smeltery-guest-guard.rs': ('transfer_selected_with_tags(', 3),
            'source-smeltery-guest-button.rs': ('transfer_selected_with_tags(', 2),
        }
        for name, (call, count) in counts.items():
            source = (ROOT / name).read_text()
            self.assertIn('forge:bottles/splash', source, name)
            self.assertIn('forge:bottles/lingering', source, name)
            self.assertEqual(source.count(call), count, name)
            for args in source.split(call)[1:]:
                self.assertIn('bottle_tags.as_ref()', args.split(');', 1)[0], name)


if __name__ == '__main__':
    unittest.main()
