"""Station rendering extraction must not rewrite its non-crop policy."""
from pathlib import Path
import hashlib
import json
import unittest

ROOT = Path(__file__).resolve().parents[2]
DRAW = ROOT / 'tconstruct/mixins/tconstruct/part-draw.rs'


def normalize_policy(source):
    newer = 'if let Some(region) = horizon_client::pluto_render::sprite(atlas, "gui/tconstruct_worktable")' in source
    r = 'region' if newer else 'mut r'
    entries = [
        ('worktable', 'if let Some(' + ('region) = horizon_client::pluto_render::sprite(atlas, "gui/tconstruct_worktable")' if newer else 'mut region) = atlas.sprites.get("gui/tconstruct_worktable")'), ''),
        ('part', 'if let Some(' + r + ')', 'let part ='),
        ('cover', 'if let Some(' + r + ')', '// Source TinkerStationScreen'),
        ('region', 'let region = |', '// Source TinkerStationScreen'),
        ('name_field', 'if let Some(' + r + ')', 'if crate::game::tconstruct_name_ui::available'),
        ('button', 'if let Some(' + r + ')', 'let hovered = (ox + x..'),
        ('empty_icons', 'if let Some(icon)', 'crate::game::tconstruct_builder_info_ui::draw'),
        ('scroll', 'if let Some(' + r + ')', 'let patterns ='),
    ]
    for name, marker, after in entries:
        start = source.index(marker, source.index(after) if after else 0)
        opening = source.index('{', start)
        end = opening + 1
        level = 1
        while level:
            if source[end] == '{': level += 1
            if source[end] == '}': level -= 1
            end += 1
        source = source[:start] + '<crop:' + name + '>' + source[end:]
    source = source.replace('if let Some(sprite) = atlas.sprites.get(texture).copied()', 'if let Some(sprite) = lookup(texture)')
    source = source.replace('if let Some(sprite) = horizon_client::pluto_render::sprite(atlas, texture)', 'if let Some(sprite) = lookup(texture)')
    return source


class RenderStationConsumers(unittest.TestCase):
    def test_all_non_crop_policy_is_byte_preserved(self):
        native = ROOT / 'pluto-lib/tests/native/rendering'
        evidence = json.loads((native / 'station-oracle-sources.json').read_text())
        source = DRAW.read_text()
        self.assertEqual(hashlib.sha256(normalize_policy(source).encode()).hexdigest(), evidence['non_crop_policy_sha256'])
        self.assertNotIn('atlas.sprites.get', source)
        for member in ('.u0', '.u1', '.v0', '.v1'):
            self.assertNotIn(member, source)
        self.assertEqual(evidence['oracle_sha256'], hashlib.sha256((native / 'station-oracle.rs').read_bytes()).hexdigest())

    def test_in_crop_choices_remain_explicit_and_local(self):
        source = DRAW.read_text()
        for choice in (
            'game.container.and_then(|pos| game.world.chunks.__horizon_mixins_tconstruct_part_selections().get(&pos)).is_some_and(|selected| selected.as_str() == *id)',
            '[176.0, if selected { 33.0 } else { 15.0 }, 18.0, 18.0]',
            '[if selected { 144.0 } else if hovered { 216.0 } else { 180.0 }, 216.0, 18.0, 18.0]',
            'for (slot, sx, sy, x) in [(0, 0.0, 216.0, 7.0), (1, 54.0, 234.0, 28.0)]',
            'if slots.get(slot).and_then(Option::as_ref).is_none()',
            '[if patterns.len() > 16 { 176.0 } else { 188.0 }, 0.0, 12.0, 15.0]',
            '(59.0 * progress.clamp(0.0, 1.0)) as i32 as f32',
            'horizon_client::hud::rgba(255, 255, 255, 209)',
        ):
            self.assertIn(choice, source)


if __name__ == '__main__':
    unittest.main()
