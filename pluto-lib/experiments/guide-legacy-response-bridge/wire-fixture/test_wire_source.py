"""Pure source render regressions, not Rust or reply execution."""
import json
import os
from pathlib import Path
import unittest
import render_wire as render


def source():
    root = Path(os.environ["GUIDE_WIRE_INPUTS"])
    bridge = Path(os.environ["GUIDE_WIRE_BRIDGE"])
    native = root / "tconstruct/mixins/tconstruct"
    return {"old_response": (native / "source-materials-book-response.rs").read_bytes(),
            "wire": (native / "source-materials-book-wire.rs").read_bytes(),
            "identity": (native / "source-materials-book-identity.rs").read_bytes(),
            "bridge": (bridge / "native-bridge.rs").read_bytes(),
            "consumer": (bridge / "consumer.rs").read_bytes(),
            "tests": (render.HERE / "regression_tests.rs").read_bytes()}


class WireSourceTests(unittest.TestCase):
    def test_complete_frozen_render(self):
        files, proof = render.compose(source())
        self.assertEqual(len(files), 7)
        self.assertFalse(proof["native_executed"])
        self.assertEqual(len(proof["expected_tests"]["regression_tests"]), 10)

    def test_wire_identity_and_retained_view_are_unmodified(self):
        original = source(); files, proof = render.compose(original)
        self.assertEqual(files["tconstruct_materials_book_wire.rs"], original["wire"])
        self.assertEqual(files["tconstruct_materials_book_identity.rs"], original["identity"])
        self.assertIn(render.unique_slice(original["bridge"].decode(), "pub(super) struct RetainedView", "#[cfg(test)]").encode(), files["bridge.rs"])

    def test_every_input_drift_and_extra_input_rejected(self):
        for key in source():
            modified = source(); modified[key] += b"\n"
            with self.assertRaises(ValueError): render.compose(modified)
        modified = source(); modified["extra"] = b""
        with self.assertRaises(ValueError): render.compose(modified)

    def test_missing_and_duplicate_source_boundaries_rejected(self):
        for value in ["begin", "begin end begin", "begin end end"]:
            with self.assertRaises(ValueError): render.unique_slice(value, "begin", "end")

    def test_substitutions_have_full_inverse(self):
        self.assertEqual(render.substitute("a before b", [("before", "after")]), "a after b")
        for raw in ["before before", "before after", "neither"]:
            with self.assertRaises(ValueError): render.substitute(raw, [("before", "after")])

    def test_no_mock_native_text_or_game_adapter(self):
        files, _ = render.compose(source())
        all_source = b"\n".join(files.values())
        self.assertIn(b"pub use horizon_client_core::text::*", all_source)
        self.assertIn(b"pluto_inventory_api::client_payload::Payload", all_source)
        for value in [b"struct Text", b"struct Game", b"game.book", b"get_or_init", b"native_client_packet("]:
            self.assertNotIn(value, all_source)


if __name__ == "__main__": unittest.main()
