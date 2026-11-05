from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'


class ContainerSlotPreservation(unittest.TestCase):
    def test_local_and_guest_container_branches_use_preserving_commit(self):
        for filename, call in (('source-smeltery-click.rs', 'transfer_selected_with_color('), ('source-smeltery-guest-guard.rs', 'transfer_selected_with_tags(')):
            source = (ROOT / filename).read_text()
            self.assertEqual(source.count('state.tank_to_nbt_preserving('), 3)
            transfers = source.split(call)[1:]
            self.assertEqual(len(transfers), 3)
            for body in transfers:
                commit = body.index('state.tank_to_nbt_preserving(')
                snapshot = body.index('.as_ref() == Some(&', commit)
                write = body.index('::write(', snapshot)
                update = body.index('_containers_mut().insert(')
                self.assertLess(commit, snapshot)
                self.assertLess(snapshot, write)
                self.assertLess(write, update)
                self.assertNotIn('state.to_nbt()', body[:update])

    def test_guest_quick_extraction_keeps_failed_commit_rollback(self):
        source = (ROOT / 'source-smeltery-guest-guard.rs').read_text()
        branch = source.split('if (0..2).contains(&packet.slot)', 1)[1].split('let mut carried', 1)[0]
        self.assertIn('else { next = containers.clone(); }', branch)
        self.assertLess(branch.index('state.tank_to_nbt_preserving(&tag)'), branch.index('hub.guests[gi].inventory = inventory'))
        self.assertIn('hub.send_menu_content', branch)


if __name__ == '__main__':
    unittest.main()
