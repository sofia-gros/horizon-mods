"""Protect the host wrappers required by the shared component transport."""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]


class TransportDelegation(unittest.TestCase):
    def test_host_and_guest_wrappers_are_not_overridden(self):
        manifest = tomllib.loads((ROOT / 'mixins/tconstruct/horizon-mixin.toml').read_text())
        targets = {entry.get('target') for entry in manifest.get('injection', [])}
        self.assertNotIn('horizon_client_lib::lan::hub::Hub::stack', targets)
        self.assertNotIn("horizon_client::game::remote::Game<'_>::to_network", targets)
        self.assertFalse((ROOT / 'mixins/tconstruct/network-custom-data.rs').exists())
        self.assertFalse((ROOT / 'mixins/tconstruct/client-network-custom-data.rs').exists())


if __name__ == '__main__':
    unittest.main()
