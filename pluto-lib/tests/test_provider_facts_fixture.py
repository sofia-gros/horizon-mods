"""Bounded facts tests must use unchanged native source and real staged policy."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class ProviderFactsFixture(unittest.TestCase):
    def test_complete_official_advancements_is_a_dependency_not_a_dto_stub(self):
        source = (ROOT / 'tools/prepare_provider_facts_fixture.py').read_text()
        self.assertIn("(native / 'advancements.rs').write_bytes(original)", source)
        self.assertIn('pub use horizon_client_core::{potions as brewing, nbt_path};', source)
        self.assertIn("(output / 'native/advancements.rs').read_bytes() != original", source)
        self.assertNotIn('pub struct ItemView', source)

    def test_module_api_and_native_package_identities_are_checked(self):
        source = (ROOT / 'tools/prepare_provider_facts_fixture.py').read_text()
        for check in ("modules[module_id]['source'].encode() != source",
                      "check_hashes(api, api_entries[0]['files'])",
                      "check_hashes(stage, evidence['stage_files_sha256'])",
                      "path.is_relative_to(stage)",
                      "actual - official - LOCAL",
                      "digest(archive.read_bytes()) != cutoff['archive_sha256']"):
            self.assertIn(check, source)


if __name__ == '__main__':
    unittest.main()
