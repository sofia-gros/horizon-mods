"""Keep bounded lifesteal and food hooks connected during integration."""
import pathlib
import tomllib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]


class NecroticTastyIntegration(unittest.TestCase):
    def test_bounded_hooks_present_and_melee_pvp_deferred(self):
        manifest = tomllib.loads((ROOT / 'mixins/tconstruct/horizon-mixin.toml').read_text())
        injections = {entry['id'] for entry in manifest['injection']}
        self.assertTrue({
            'source-tasty-use', 'source-tasty-duration', 'source-tasty-animation',
            'source-tasty-finish', 'source-tasty-concurrent-tick',
            'source-necrotic-head', 'source-necrotic-return', 'source-necrotic-hit',
        }.issubset(injections))
        self.assertFalse(any('necrotic-melee-pvp' in name for name in injections))


    def test_necrotic_melee_wear_and_projectile_deferral(self):
        source = (ROOT / 'mixins/tconstruct/source-necrotic.rs').read_text()
        manifest = tomllib.loads((ROOT / 'mixins/tconstruct/horizon-mixin.toml').read_text())
        injections = {entry['id'] for entry in manifest['injection']}
        self.assertIn('Some("tconstruct:necrotic")', source)
        self.assertIn('damage_with_cause(', source)
        self.assertFalse({'source-necrotic-projectile-begin', 'source-necrotic-projectile-finish',
                          'source-necrotic-pvp-begin', 'source-necrotic-pvp-end'} & injections)


    def test_station_checks_melee_tag_and_tooltip_warns(self):
        station = (ROOT / 'mixins/tconstruct/source-station-world.rs').read_text()
        tooltip = (ROOT / 'mixins/tconstruct/part-name-injection.rs').read_text()
        reject = station.index('Necrotic is currently available only on held melee weapons')
        plan = station.rfind('let plan =', 0, reject)
        mutate = station.index('prepare_for_tool_mutation(tool)', reject)
        self.assertGreater(reject, plan)
        self.assertLess(reject, mutate)
        self.assertIn('modifiable/melee/weapon', station[plan:reject])
        self.assertIn('Necrotic: held mob melee only; worn, PvP and projectiles unavailable', tooltip)
        # These source items are intentionally melee-tagged and can still use
        # the bounded mob-melee effect, despite also having ranged actions.
        import json
        tag = json.loads((ROOT / 'content/reference/tconstruct-tags/data/tconstruct/tags/items/modifiable/melee/weapon.json').read_text())
        self.assertTrue({'tconstruct:longbow', 'tconstruct:crossbow', 'tconstruct:fishing_rod'} <= set(tag['values']))


if __name__ == '__main__':
    unittest.main()
