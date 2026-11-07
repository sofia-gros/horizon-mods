"""The render fixture may not silently resolve a different official dependency."""
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

PATH = Path(__file__).resolve().parents[1] / 'tools/prepare_render_fixture.py'
SPEC = importlib.util.spec_from_file_location('render_fixture', PATH)
FIXTURE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(FIXTURE)


class RenderFixtureIdentity(unittest.TestCase):
    def test_only_exact_fixture_addition_is_allowed(self):
        host = {'name': 'glam', 'version': '0.30.10', 'source': 'registry', 'checksum': 'abc'}
        fixture = {'name': 'pluto-render-equivalence', 'version': '0.0.0'}
        FIXTURE.check_identities({'package': [host]}, {'package': [host, fixture]})
        # An unused host dependency may disappear from a focused fixture.
        FIXTURE.check_identities({'package': [host]}, {'package': [fixture]})
        for change in ({'version': '0.30.11'}, {'source': 'other'}, {'checksum': 'def'}):
            with self.assertRaises(ValueError):
                FIXTURE.check_identities({'package': [host]}, {'package': [host | change, fixture]})
        with self.assertRaises(ValueError):
            FIXTURE.check_identities({'package': [host]}, {'package': [host, fixture | {'version': '0.0.1'}]})
        with self.assertRaises(ValueError):
            FIXTURE.check_identities({'package': [host]}, {'package': [host]})

    def test_clean_official_identity_is_required(self):
        with patch.object(FIXTURE.subprocess, 'check_output', side_effect=[FIXTURE.OFFICIAL_ORIGIN, '624c', '']):
            self.assertEqual(FIXTURE.clean_official(Path('/approved'), '624c'), '624c')
        for outputs in ([FIXTURE.OFFICIAL_ORIGIN, 'wrong', ''],
                        [FIXTURE.OFFICIAL_ORIGIN, '624c', ' M Cargo.lock'],
                        ['https://example.invalid/other', '624c', '']):
            with patch.object(FIXTURE.subprocess, 'check_output', side_effect=outputs), self.assertRaises(ValueError):
                FIXTURE.clean_official(Path('/approved'), '624c')


if __name__ == '__main__':
    unittest.main()
