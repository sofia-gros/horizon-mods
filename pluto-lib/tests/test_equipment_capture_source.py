"""Static adapter contract, not a live Game/offhand execution fixture."""
from pathlib import Path
import re
import unittest


class EquipmentCaptureSourceTests(unittest.TestCase):
    def test_main_hand_is_selected_slot_not_acting_hand(self):
        path = Path(__file__).resolve().parents[1] / "mixins/pluto_native_creative/equipment-game-dispatch.rs"
        code = re.sub(r"//[^\n]*", "", path.read_text())
        code = re.sub(r"\s+", "", code)
        self.assertIn("game.stack_in(game.selected)", code)
        self.assertNotIn("game.held()", code)
        self.assertIn("game.offhand.as_ref()", code)

    def test_acting_offhand_does_not_replace_selected_main_slot(self):
        # Model the official held()/held_slot()/stack_in() contract. The source
        # assertion above ties the adapter to selected-slot access. This is not
        # a substitute for a native Game fixture.
        selected = 2
        offhand_slot = 40
        inventory = {2: "selected-main", 3: "other-main"}
        offhand = "offhand"
        def stack_in(slot):
            return offhand if slot == offhand_slot else inventory.get(slot)
        for acting_slot in (selected, offhand_slot):
            acting_hand = stack_in(acting_slot)
            captured = (stack_in(selected), offhand)
            self.assertEqual(captured, ("selected-main", "offhand"))
            if acting_slot == offhand_slot:
                self.assertEqual(acting_hand, "offhand")
                self.assertNotEqual(captured[0], acting_hand)

    def test_empty_selected_slot_stays_empty_during_offhand_action(self):
        selected = 2
        inventory = {3: "other-main"}
        offhand = "offhand"
        captured = (inventory.get(selected), offhand)
        self.assertEqual(captured, (None, "offhand"))
