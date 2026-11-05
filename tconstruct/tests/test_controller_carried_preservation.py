from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'


class ControllerCarriedPreservation(unittest.TestCase):
    def test_local_and_guest_use_preserving_save_and_exact_index(self):
        text = (ROOT / 'source-fluid-direct.rs').read_text().split('/// Stage and commit a held-bucket', 1)[0]
        for body in (text.split('pub(super) fn local(', 1)[1].split('pub(super) fn guest(', 1)[0], text.split('pub(super) fn guest(', 1)[1]):
            self.assertIn('fill_from_index_with_color(&mut state.tank, &mut slots, index', body)
            self.assertNotIn('fluid.as_deref()', body)
            self.assertNotIn('state.to_nbt()', body)
            self.assertIn('data.wire.patch().is_some()', body)
            self.assertIn('let Some(saved) = state.tank_to_nbt_preserving(&raw)', body)
            write = body.index('"tconstruct:smeltery", saved)')
            self.assertLess(body.index('.as_ref() != Some(&raw)'), write)
            self.assertLess(write, body.index('.carried ='))
        guest = text.split('pub(super) fn guest(', 1)[1]
        self.assertEqual(guest.count('tags.as_ref(), false)'), 2)
        self.assertLess(guest.index('"tconstruct:smeltery", saved)'), guest.index('SetCursorItem'))


if __name__ == '__main__':
    unittest.main()
