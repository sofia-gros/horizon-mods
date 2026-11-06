"""Installed source/package contracts, not a renderer or native font test."""
import importlib.util
import json
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
NATIVE = ROOT / "mixins/pluto_native_creative"


class GuideLayoutBoundaryTests(unittest.TestCase):
    def test_exact_public_module_has_no_automatic_hook_or_state(self):
        manifest = tomllib.loads((NATIVE / "horizon-mixin.toml").read_text())
        modules = [m for m in manifest["module"] if m["id"] == "guide-layout"]
        self.assertEqual(modules, [{"id": "guide-layout", "file": "crates/horizon-client/src/client_lib.rs",
                                   "name": "pluto_guide_layout", "source": "guide-layout.rs", "visibility": "pub"}])
        for section in ("injection", "state"):
            self.assertFalse(any("guide-layout" in m["id"] for m in manifest.get(section, [])))

    def test_packaged_bytes_are_actual_module_and_converter(self):
        spec = importlib.util.spec_from_file_location("layout_package", ROOT / "tools/package_native.py")
        package = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(package)
        _, files = package._bundle(json.loads((ROOT / "horizon-native.json").read_text()))
        for name in ("guide-layout.rs", "guide-text.rs"):
            self.assertEqual(files[f"pluto_native_creative/{name}"], (NATIVE / name).read_bytes())
            self.assertFalse(any(line.lstrip().startswith(("//!", "#!")) for line in (NATIVE / name).read_text().splitlines()))

    def test_work_admission_precedes_native_style_expansion(self):
        source = (NATIVE / "guide-layout.rs").read_text()
        body = source[source.index("pub fn wrap("):]
        self.assertLess(body.index("admit(page)?"), body.index("to_native_page(page)"))
        self.assertLess(body.index("admit(page)?"), body.index("font.split("))
        for forbidden in ("Game<'", "CustomPayload", "open_book(", "turn_page(", "static CALLBACKS", "tconstruct:"):
            self.assertNotIn(forbidden, source)


if __name__ == "__main__":
    unittest.main()
