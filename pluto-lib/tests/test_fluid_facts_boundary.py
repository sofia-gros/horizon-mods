"""Source-boundary checks, not native compilation or gameplay proof."""
from pathlib import Path
import re
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
NATIVE = ROOT / "pluto-lib/mixins/pluto_native_creative"
CONSUMER = ROOT / "tconstruct/mixins/tconstruct"


class FluidFactsBoundary(unittest.TestCase):
    def test_support_module_uses_client_library_without_new_hooks(self):
        manifest = tomllib.loads((NATIVE / "horizon-mixin.toml").read_text())
        matches = [row for row in manifest["module"] if row["id"] == "fluid-facts"]
        self.assertEqual(matches, [{
            "id": "fluid-facts", "file": "crates/horizon-client/src/client_lib.rs",
            "name": "pluto_fluid_facts", "source": "fluid-facts.rs", "visibility": "pub",
        }])
        for section in ("injection", "state", "crate"):
            self.assertFalse(any(row["id"] == "fluid-facts" for row in manifest.get(section, [])))

    def test_native_access_has_one_shared_owner_and_no_consumer_policy(self):
        source = (NATIVE / "fluid-facts.rs").read_text()
        for expression in ("state.kind().name()", "state.is_full()", "FluidKind::ALL",
                           "behaviours.legacy_block(behaviours.fluid(state))",
                           "player.is_eye_in_fluid(WATER)", "player.fluids.was_eye_in_water",
                           "player.fluids.was_touching_water"):
            self.assertIn(expression, source)
        self.assertNotIn("tconstruct", source.lower())
        self.assertNotIn("pluto_api", source)
        self.assertNotIn("set_block", source)

    def test_five_consumers_no_longer_decode_native_fluid_state(self):
        for name in ("source-native-hook-bodies.rs", "source-ender-propagule.rs",
                     "source-splash-conditional.rs", "source-mining-context-shared.rs",
                     "source-projectile-airborne.rs"):
            source = (CONSUMER / name).read_text()
            with self.subTest(name=name):
                self.assertIn("horizon_client::pluto_fluid_facts::", source)
                self.assertIsNone(re.search(
                    r"FluidKind|\.fluid\(|\.legacy_block\(|\.fluids\.|is_eye_in_fluid|motion::WATER",
                    source))

    def test_consumer_identity_and_sample_distinctions_stay_explicit(self):
        native = (CONSUMER / "source-native-hook-bodies.rs").read_text()
        self.assertIn('fluid.id == "minecraft:water"', native)
        self.assertIn('old_name == "minecraft:water", fluid.full', native)
        self.assertIn("pluto_fluid_facts::native_ids()", native)
        mining = (CONSUMER / "source-mining-context-shared.rs").read_text()
        self.assertIn("water.is_some_and(|facts| facts.eye_in_water)", mining)
        self.assertIn("facts.previous_eye_in_water.then_some(aqua_affinity)", mining)
        projectile = (CONSUMER / "source-projectile-airborne.rs").read_text()
        self.assertIn("pluto_fluid_facts::player(player).touching_water", projectile)


if __name__ == "__main__":
    unittest.main()
