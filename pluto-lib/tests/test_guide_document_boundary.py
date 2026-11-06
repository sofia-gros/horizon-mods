"""Source/package boundary tests; no native screen or packet execution."""
import importlib.util
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
NATIVE = ROOT / "mixins/pluto_native_creative"


class GuideDocumentBoundaryTests(unittest.TestCase):
    def test_codec_has_no_consumer_identity_or_native_dependency(self):
        source = (NATIVE / "inventory-api/src/guide/document.rs").read_text()
        for forbidden in ["tconstruct:", "Materials & You", "horizon_client::", "horizon_world::", "static CALLBACKS", "Game<'"]:
            self.assertNotIn(forbidden, source)
        self.assertIn('"version":1', source.replace(" ", ""))

    def test_native_converter_has_no_screen_input_or_packet_effect(self):
        source = (NATIVE / "guide-text.rs").read_text()
        for forbidden in ["open_book(", "close_book(", "handle_packet(", "CustomPayload", "game.send(", "Game<'", "native_client_packet("]:
            self.assertNotIn(forbidden, source)
        self.assertIn("raw > api::MAX_BYTES", source)
        self.assertIn("text.spans =", source)
        self.assertIn("Error::UnsupportedStyle", source)

    def test_document_converter_installs_no_automatic_hook_or_state(self):
        manifest = tomllib.loads((NATIVE / "horizon-mixin.toml").read_text())
        modules = [m for m in manifest["module"] if m["id"].startswith("guide-")]
        self.assertEqual({m["source"] for m in modules}, {"guide-text.rs", "guide-layout.rs"})
        for section in ["injection", "state"]:
            self.assertFalse(any(m["id"].startswith("guide-") for m in manifest.get(section, [])))

    def test_packager_includes_nested_codec_and_native_conversion(self):
        spec = importlib.util.spec_from_file_location("guide_package", ROOT / "tools/package_native.py")
        module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        import json
        _, files = module._bundle(json.loads((ROOT / "horizon-native.json").read_text()))
        for name in ["guide-text.rs", "guide-layout.rs", "inventory-api/src/guide/mod.rs", "inventory-api/src/guide/document.rs"]:
            self.assertEqual(files[f"pluto_native_creative/{name}"], (NATIVE/name).read_bytes())


if __name__ == "__main__":
    unittest.main()
