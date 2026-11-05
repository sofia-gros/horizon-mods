"""Pinned Overgrowth definition and inventory/reload wiring, not gameplay QA."""
import json
from pathlib import Path
import re
import tomllib
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
MIXINS = ROOT / "mixins/tconstruct"


class Overgrowth(unittest.TestCase):
    def test_exact_source_definition(self):
        helper = (MIXINS / "source-overgrowth.rs").read_text()
        fixture = re.search(r'let bytes\s*=\s*br#"(.*?)"#;', helper).group(1)
        with ZipFile(ROOT / "content/reference/tconstruct-modifiers.zip") as archive:
            source = archive.read("data/tconstruct/tinkering/modifiers/overgrowth.json")
        self.assertEqual(json.loads(fixture), json.loads(source))

    def test_authoritative_inventory_hook_and_per_holder_state(self):
        manifest = tomllib.loads((MIXINS / "horizon-mixin.toml").read_text())
        hook = next(v for v in manifest["injection"] if v["id"] == "source-overgrowth-tick")
        self.assertEqual(hook["kind"], "TAIL")
        self.assertEqual(hook["target"], "horizon_client::game::native_hooks::Game<'_>::native_inventory_ticks")
        state = next(v for v in manifest["state"] if v["id"] == "overgrowth-phase")
        self.assertEqual(state["target"], "horizon_client_render::chunk_map::ChunkMap")
        self.assertEqual(state["ty"], "std::collections::HashMap<u128, u8>")
        helper = (MIXINS / "source-overgrowth-world.rs").read_text()
        for marker in ["game.remote.is_some()", "game.native_acting_id()", ".entry(actor)",
                       "rule::active_slot(slot, using)", "__horizon_mixins_tconstruct_modifier_resource_snapshot()",
                       "tconstruct:modifiable", "tconstruct:overworked", 'fields.insert("tic_overslime"']:
            self.assertIn(marker, helper)
        # The current snapshot is read on each recovery pass, not captured once.
        self.assertLess(helper.index("rule::known("), helper.index("for slot in slots"))
        self.assertLess(helper.index("rule::eligible("), helper.index("game.world.random.next_float()"))


if __name__ == "__main__":
    unittest.main()
