from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'


class SmelteryMenuPreservation(unittest.TestCase):
    def test_local_and_guest_changes_patch_only_tank(self):
        for filename in ('source-smeltery-pointer.rs', 'source-smeltery-guest-button.rs'):
            source = (ROOT / filename).read_text()
            self.assertNotIn('state.to_nbt()', source)
            self.assertEqual(source.count('state.tank_to_nbt_preserving(&raw)'), 2)
            self.assertEqual(source.count('.as_ref() != Some(&raw)'), 2)
            branches = source.split('let Some(saved) = state.tank_to_nbt_preserving(&raw)')[1:]
            for body in branches:
                read = body.index('.as_ref() != Some(&raw)')
                write = body.index('"tconstruct:smeltery", saved)')
                update = body.index('_containers_mut().insert(')
                self.assertLess(read, write)
                self.assertLess(write, update)

    def test_controller_codec_refuses_unrelated_state_changes(self):
        source = (ROOT / 'source-smeltery-state.rs').read_text()
        body = source.split('pub(super) fn tank_to_nbt_preserving(', 1)[1].split('pub(super) fn from_nbt(', 1)[0]
        self.assertIn('unchanged.to_nbt() != before.to_nbt()', body)
        self.assertIn('self.tank.to_nbt_preserving(raw.get("tank")?)', body)
        self.assertIn('raw.clone()', body)
        self.assertIn('fields.insert("tank".into(), tank)', body)


if __name__ == '__main__':
    unittest.main()
