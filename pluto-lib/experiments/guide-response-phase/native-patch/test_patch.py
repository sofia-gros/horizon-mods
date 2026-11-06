"""Source/hunk regressions only; require an explicitly supplied official tree."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("retained_patch", HERE / "generate.py")
gen = importlib.util.module_from_spec(spec); spec.loader.exec_module(gen)


class PrototypePatchTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if "PLUTO_OFFICIAL_SOURCE" not in os.environ:
            raise RuntimeError("Set PLUTO_OFFICIAL_SOURCE to the explicitly approved exact official source tree")
        cls.official = Path(os.environ["PLUTO_OFFICIAL_SOURCE"])
        cls.pins = json.loads((HERE / "official-sources.json").read_text())
        cls.inputs = gen.read_inputs(cls.official, cls.pins)
        cls.output, cls.rewrites = gen.transform(cls.inputs)

    def test_exact_generated_patch_applies_only_in_memory(self):
        patch, receipt = gen.generate(self.official)
        self.assertEqual(patch, (HERE / "prototype.patch").read_text())
        self.assertEqual(receipt, json.loads((HERE / "generation.json").read_text()))
        self.assertEqual(gen.apply_in_memory(patch, self.inputs), self.output)
        self.assertEqual(len(receipt["changed_files"]), 11)

    def test_real_game_field_and_constructor_use_original_book_screen(self):
        source = self.output[gen.PREFIX + "game.rs"]
        self.assertIn("book: retained_book::RetainedBook<books::BookScreen>", source)
        self.assertIn("book: retained_book::RetainedBook::default()", source)
        books = self.output[gen.PREFIX + "game/books.rs"]
        self.assertEqual(books[books.index("pub(super) enum BookScreen"):books.index("/// The pages a book screen reads")],
                         self.inputs[gen.PREFIX + "game/books.rs"][self.inputs[gen.PREFIX + "game/books.rs"].index("pub(super) enum BookScreen"):self.inputs[gen.PREFIX + "game/books.rs"].index("/// The pages a book screen reads")])

    def test_raw_option_writes_and_pattern_borrows_are_all_routed(self):
        for path, source in self.output.items():
            self.assertNotRegex(source, r"self\.book\s*=|&mut self\.book\b|&self\.book\b|self\.book,", path)
        self.assertIn("self.book.replace(host_book)", self.output[gen.PREFIX + "game/lan_actions.rs"])
        self.assertIn("=> self.book.set(Some(*book)),", self.output[gen.PREFIX + "game/remote_chat.rs"])

    def test_dialog_overlay_and_native_packet_observer_are_not_replaced(self):
        self.assertEqual(self.output[gen.PREFIX + "game/dialogs.rs"], self.inputs[gen.PREFIX + "game/dialogs.rs"])
        native = self.output[gen.PREFIX + "game/native_hooks.rs"]
        self.assertIn("mod_api::dispatch_client_packet(self, mod_api::mods(), packet);", native)
        remote = self.output[gen.PREFIX + "game/remote.rs"]
        self.assertIn("pub(super) fn handle_packet(&mut self, packet: C) -> Result<()> {\n        self.native_client_packet(&packet);", remote)
        self.assertNotIn("tconstruct", (HERE / "retained_book.rs").read_text().lower())

    def test_existing_native_borrow_sites_are_preserved_not_suppressed(self):
        old = self.inputs[gen.PREFIX + "game/books.rs"]
        new = self.output[gen.PREFIX + "game/books.rs"]
        self.assertEqual(old.count("self.book.as_mut()"), new.count("self.book.as_mut()"))
        self.assertEqual(old.count("self.book.take()"), new.count("self.book.take()"))
        self.assertEqual(old.count("self.play_ui("), new.count("self.play_ui("))
        self.assertIn("self.value.as_mut()", (HERE / "retained_book.rs").read_text())

    def test_count_or_hunk_context_drift_fails_closed(self):
        changed = copy.deepcopy(self.inputs)
        changed[gen.PREFIX + "game/books.rs"] += "\nself.book = None;\n"
        with self.assertRaisesRegex(ValueError, "rewrite count drift"):
            gen.transform(changed)
        original = (HERE / "prototype.patch").read_text()
        patch = original.replace(" mod books;\n", " mod wrong;\n", 1)
        self.assertNotEqual(patch, original)
        with self.assertRaisesRegex(ValueError, "context mismatch"):
            gen.apply_in_memory(patch, self.inputs)

    def test_attachment_and_book_access_inventory_is_explicit(self):
        self.assertEqual(sum(len(x["accesses"]) for x in self.pins["book_field_inventory"]), 79)
        transition_counts = {r["kind"]: sum(x["count"] for x in self.rewrites if x.get("kind") == r["kind"])
                             for r in self.rewrites if "kind" in r}
        self.assertEqual(transition_counts, {"actual-player-assignment": 8, "actual-remote-take": 3})
        self.assertIn("game/native_machines.rs", self.pins["excluded_attachment_paths"])


if __name__ == "__main__":
    unittest.main()
