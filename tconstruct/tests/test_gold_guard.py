"""Gold Guard source definitions and authoritative attribute/reload ownership."""
import json
from pathlib import Path
import re
import tomllib
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
MIXINS = ROOT / 'mixins/tconstruct'


class GoldGuard(unittest.TestCase):
    def test_exact_source_definitions(self):
        helper = (MIXINS / 'source-gold-guard.rs').read_text()
        with ZipFile(ROOT / 'content/reference/tconstruct-modifiers.zip') as archive:
            for name, variable in [('gold_guard', 'guard'), ('golden', 'golden')]:
                source = json.loads(archive.read(f'data/tconstruct/tinkering/modifiers/{name}.json'))
                fixture = re.search(rf'let {variable}=br#"(.*?)"#;', helper).group(1)
                self.assertEqual(json.loads(fixture), source)

    def test_host_target_and_reload_publication_boundary(self):
        manifest = tomllib.loads((MIXINS / 'horizon-mixin.toml').read_text())
        hook = next(v for v in manifest['injection'] if v['id'] == 'source-gold-guard-attributes')
        self.assertEqual(hook['kind'], 'RETURN')
        self.assertEqual(hook['target'], "horizon_client::game::enchant_effects::Game<'_>::apply_enchantment_attributes")
        reload = (MIXINS / 'recipe-reload.rs').read_text()
        self.assertLess(reload.index('__horizon_mixins_tconstruct_modifier_resource_snapshot_mut() = Some(modifiers)'),
                        reload.index('tconstruct_gold_guard_world::refresh(game)'))
        helper = (MIXINS / 'gold-guard-world.rs').read_text()
        self.assertIn('game.remote.is_some()', helper)
        self.assertIn('has_modifier(key.clone(),source::MODIFIER)', helper)
        self.assertIn('player.set_health(maximum,false)', helper)
        self.assertNotIn('applied_modifiers.push', helper)


if __name__ == '__main__':
    unittest.main()
