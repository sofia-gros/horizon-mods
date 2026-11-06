"""Source boundaries only, not native UI execution or host compilation proof."""
from pathlib import Path
import re
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
NATIVE = ROOT / "mixins/pluto_native_creative"


class RecipeBookInvalidationTests(unittest.TestCase):
    def test_exact_existing_invalidation_mechanics_are_shared(self):
        # Both reviewed original consumer helpers have exactly this body:
        # fake material 8c56bb52 and Travelers a8fc1e18. Their trigger conditions
        # and unrelated preview methods are deliberately not imported here.
        expected = """
            let game = &mut *self.0;
            game.forget_book();
            game.recipe_ui.client_known = None;
            game.recipe_ui.view = Default::default();
            game.recipe_ui.ghost.clear();
            game.recipe_ui.overlay = None;
            game.recipe_ui.last_placed = None;
            game.recipe_ui.last_clicked = None;
        """
        source = (NATIVE / "recipe-book-invalidation.rs").read_text()
        body = source.split("fn invalidate_definitions(&mut self) {", 1)[1].split("}", 1)[0]
        compact = lambda text: re.sub(r"\s+", "", re.sub(r"//[^\n]*", "", text))
        self.assertEqual(compact(body), compact(expected))

    def test_only_one_private_native_module_no_hooks_or_owned_state(self):
        manifest = tomllib.loads((NATIVE / "horizon-mixin.toml").read_text())
        matches = [row for row in manifest["module"] if row["id"] == "recipe-book-invalidation"]
        self.assertEqual(matches, [{
            "id": "recipe-book-invalidation",
            "file": "crates/horizon-client/src/game/recipe_book.rs",
            "name": "pluto_invalidation",
            "source": "recipe-book-invalidation.rs",
            "visibility": "pub(crate)",
        }])
        for section in ("injection", "state", "crate"):
            self.assertFalse(any(row["id"] == "recipe-book-invalidation"
                                 for row in manifest.get(section, [])))
        self.assertFalse((NATIVE / "recipe-book-invalidation.rs").read_text().startswith("//!"))

    def test_public_capability_has_no_native_types_or_mutation_policy(self):
        source = (NATIVE / "inventory-api/src/recipe_book.rs").read_text()
        self.assertIn("pub trait Invalidation", source)
        self.assertIn("fn invalidate_definitions(&mut self);", source)
        code = re.sub(r"//[^\n]*", "", source)
        for native in ("horizon_", "Game", "ItemStack", "static", "register", "tconstruct"):
            self.assertNotIn(native, code)
        self.assertIn("pub mod recipe_book;", (NATIVE / "inventory-api/src/lib.rs").read_text())


if __name__ == "__main__":
    unittest.main()
