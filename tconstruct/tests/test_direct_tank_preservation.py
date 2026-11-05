from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'


class DirectTankPreservation(unittest.TestCase):
    def test_exchange_helpers_preserve_raw_state_and_recheck_before_save(self):
        source = (ROOT / 'source-fluid-direct.rs').read_text()
        for name, next_name in (('melter_exchange', 'melter_local'), ('fuel_exchange', 'fuel_local')):
            body = source.split(f'fn {name}(', 1)[1].split(f'pub(super) fn {next_name}(', 1)[0]
            self.assertIn('containers::plan_single_tank_container(', body)
            self.assertNotIn('.to_nbt()', body)
            for check in ('contains_key(', '!= block', '.as_ref() != Some(&raw)'):
                self.assertLess(body.index(check), body.index('::write('))
        fuel = source.split('fn fuel_exchange(', 1)[1].split('pub(super) fn fuel_local(', 1)[0]
        self.assertIn('_smeltery_owners().get(&at) != Some(&pos)', fuel)
        melter = source.split('fn melter_exchange(', 1)[1].split('pub(super) fn melter_local(', 1)[0]
        self.assertIn('raw.clone()', melter)
        self.assertIn('fields.insert("tank".into(), tank)', melter)

    def test_planner_rejects_wire_aliases_and_validates_identity(self):
        body = (ROOT / 'source-fluid-container.rs').read_text().split('pub(super) fn plan_single_tank_container(', 1)[1].split('/// Stage a direct Melter interaction', 1)[0]
        self.assertIn('data.wire.patch().is_some()', body)
        self.assertIn('casting_portable_rewrite_safe(item)', body)
        self.assertIn('tank.to_nbt_preserving(raw).as_ref() != Some(raw)', body)
        self.assertIn('Some((tank.to_nbt_preserving(raw)?, slots))', body)


if __name__ == '__main__':
    unittest.main()
