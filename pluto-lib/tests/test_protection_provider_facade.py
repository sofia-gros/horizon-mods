"""Facade ownership and adapter source contracts, not native host execution."""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1] / "mixins/pluto_native_creative"


class ProtectionProviderFacadeTests(unittest.TestCase):
    def test_single_registry_has_no_native_dependencies(self):
        api = (ROOT / "inventory-api/src/protection.rs").read_text()
        native = (ROOT / "protection-player-api.rs").read_text()
        self.assertEqual(api.count("static CALLBACKS:"), 1)
        for path in ["horizon_world::", "horizon_client::", "PlayerMotion", "ItemStack", "HitContext"]:
            self.assertNotIn(path, api)
        self.assertNotIn("static PROVIDERS", native)
        self.assertNotIn("RwLock", native)
        self.assertIn("api::enabled()", native)
        self.assertIn("api::visit(|id, provider|", native)

    def test_native_capability_always_uses_its_fixed_id(self):
        native = (ROOT / "protection-contribution.rs").read_text().split("#[cfg(test)]")[0]
        for method in ["set_lower", "set_upper", "set_additive", "remove"]:
            self.assertIn(f"self.policy.{method}(self.id", native)
        self.assertNotIn("pub id:", native)
        self.assertNotIn("pub policy:", native)

    def test_projection_reuses_converter_and_never_reads_live_game(self):
        native = (ROOT / "protection-provider-facts.rs").read_text()
        self.assertIn("pluto_component_view::ItemFacts", native)
        self.assertIn("snapshot", native)
        self.assertIn("stack.map(ItemFacts::capture)", native)
        self.assertIn("impl equipment_api::EquipmentSlots for Equipment", native)
        self.assertNotIn("fn active_use", native)
        self.assertNotIn("Game", native.split("use crate", 1)[1])
        for enum in ["Armor", "Effects", "Resistance", "Enchantments"]:
            self.assertIn(f"api::Bypass::{enum} => Bypass::{enum}", native)

    def test_replaced_native_hit_context_is_not_packaged(self):
        manifest = tomllib.loads((ROOT / "horizon-mixin.toml").read_text())
        modules = {item["id"]: item for item in manifest["module"]}
        self.assertNotIn("damage-hit-context", modules)
        self.assertEqual(modules["protection-provider-facts"]["source"], "protection-provider-facts.rs")
        self.assertEqual(modules["protection-contribution"]["source"], "protection-contribution.rs")
        self.assertFalse((ROOT / "damage-hit-context.rs").exists())


if __name__ == "__main__":
    unittest.main()
