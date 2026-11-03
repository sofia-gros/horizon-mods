"""Source guards for the Melter tooltip's menu position; not gameplay tests."""
from pathlib import Path
import unittest

MIXINS = Path(__file__).resolve().parents[1] / 'mixins' / 'tconstruct'


class MelterRemoteTooltip(unittest.TestCase):
    def test_tooltip_uses_same_captured_position_as_renderer(self):
        def position_line(name):
            return next(line.strip() for line in (MIXINS / name).read_text().splitlines()
                        if 'let position = if game.remote.is_some()' in line)
        line = position_line('station-result-tooltip.rs')
        self.assertEqual(line, position_line('part-draw.rs'))
        self.assertIn('__horizon_mixins_tconstruct_remote_screen_position()', line)
        self.assertIn('else { game.container }', line)

    def test_screen_open_captures_target(self):
        self.assertIn('remote_screen_position_mut() = target',
                      (MIXINS / 'source-machine-remote-screen.rs').read_text())


if __name__ == '__main__':
    unittest.main()
