from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'

class MachinePotionColor(unittest.TestCase):
    def test_render_paths_require_local_complete_metadata(self):
        for file, expected in [('part-draw.rs', 2), ('source-smeltery-draw.rs', 1), ('source-machine-geometry.rs', 1)]:
            source = (ROOT / file).read_text()
            self.assertEqual(source.count('tconstruct_source_melter_render::color_with_complete('), expected, file)
            self.assertEqual(source.count('game.remote.is_none())'), expected, file)
        render = (ROOT / 'source-melter-render.rs').read_text()
        self.assertIn('if fluid == "tconstruct:potion" && metadata_complete', render)
        self.assertIn('color_tagged(fluid, tag)', render)

if __name__ == '__main__':
    unittest.main()
