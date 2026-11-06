"""Additive API scope guards, not Rust compilation or native event proof."""
import hashlib
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
API = ROOT / "mixins/pluto_native_creative/inventory-api"

def sha(text):
    return hashlib.sha256(text.encode()).hexdigest()

class GuideErgonomics(unittest.TestCase):
    def test_removing_event_additions_restores_exact_existing_registry(self):
        source = (API / "src/client_payload.rs").read_text()
        start = source.index("/// Read-only event entry for the existing")
        end = source.index("/// Subscribe during setup", start)
        original = source[:start] + source[end:]
        start = original.index("\n    #[test]\n    fn event_entry_")
        original = original[:start] + "}\n"
        self.assertEqual(sha(original), "bb21ff3ba1dcc2db508d43c1d4ace10359e823d3399e3132de6fe9f1316d7af2")

    def test_removing_constructors_restores_exact_document_contract(self):
        source = (API / "src/guide/document.rs").read_text()
        start, end = source.index("impl Span {"), source.index("pub type Page")
        self.assertEqual(sha(source[:start] + source[end:]), "5540cffa93542600cdad7f0800c29a93045a0aeaf1dcf185b18b026333f3fd12")

    def test_existing_guide_modules_still_have_identical_prefix(self):
        source = (API / "src/guide/mod.rs").read_text()
        original = source.split("\n// Short paths to the same contracts", 1)[0]
        self.assertEqual(sha(original), "549832c342d3f4e3db6bbdc058c71936381f5dd626b174543094729b2fe72e5c")
        self.assertIn("pub use document::{Document, Page, Span, Style};", source)
        self.assertIn("pub use requests::RequestTracker;", source)

    def test_event_is_configuration_and_delegation_not_another_dispatcher(self):
        source = (API / "src/client_payload.rs").read_text()
        new = source.split("/// Read-only event entry for the existing", 1)[1].split("/// Subscribe during setup", 1)[0]
        self.assertEqual(source.count("static SUBSCRIPTIONS:"), 1)
        for token in ["register(id, self.channel, self.max_bytes, callback)", "remove(id)", "channel: &'a str"]:
            self.assertIn(token, new)
        for token in ["RwLock", "BTreeMap", "Vec<", "String", "observe(payload", "Game", "unsafe"]:
            self.assertNotIn(token, new)

    def test_external_example_has_explicit_bounded_startup_and_cleanup(self):
        source = (API / "examples/guide_observer.rs").read_text()
        for token in ["use pluto_inventory_api::", "client_payload::OBSERVED", ".channel(\"example:guide_document\", 4096)", ".register(\"example:guide_logger\", log_document)", "OBSERVED.remove", "Document::new", "Span::plain", "Span::styled"]:
            self.assertIn(token, source)
        for token in ["tconstruct", "Game<", "::observe(", "send_packet", "with_current", "unsafe"]:
            self.assertNotIn(token, source)

if __name__ == "__main__":
    unittest.main()
