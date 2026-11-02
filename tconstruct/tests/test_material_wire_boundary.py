"""The chunk metadata hook covers both b2 body_with and its body_in delegate."""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]


class MaterialWireBoundary(unittest.TestCase):
    def test_hook_uses_real_proto_context_once(self):
        root = ROOT / "mixins" / "tconstruct"
        manifest = tomllib.loads((root / "horizon-mixin.toml").read_text())
        hooks = [entry for entry in manifest["injection"] if entry["id"] == "material-block-wire-chunk"]
        self.assertEqual(len(hooks), 1)
        self.assertEqual(hooks[0]["target"], "horizon_client_lib::lan::chunks::ChunkEncoder::body_with")
        self.assertTrue(any(binding["name"] == "protos" and binding["value"] == "protos" for binding in hooks[0]["bindings"]))
        helper = (root / "material-block-wire-chunk.rs").read_text()
        self.assertIn("encoder.around(chunk, world, protos)", helper)
        self.assertNotIn("encoder.around(chunk, world, &|_, _| None)", helper)


if __name__ == "__main__":
    unittest.main()
