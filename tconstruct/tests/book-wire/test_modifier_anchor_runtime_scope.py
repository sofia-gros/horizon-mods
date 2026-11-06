"""Source reconstruction guards only; the supplied Rust tests are unrun."""
import hashlib
import json
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
MOD = ROOT / "tconstruct/mixins/tconstruct"
SCOPE = json.loads((HERE / "modifier-anchor-runtime-scope.json").read_text())


def restore(record):
    source = (ROOT / record["path"]).read_text()
    if hashlib.sha256(source.encode()).hexdigest() != record["after_sha256"]:
        raise AssertionError("Candidate source drift: " + record["path"])
    for edit in reversed(record["edits"]):
        if source.count(edit["new"]) != edit["count"]:
            raise AssertionError("Inverse edit drift: " + record["path"])
        source = source.replace(edit["new"], edit["old"])
    return source


class ModifierAnchorRuntimeScope(unittest.TestCase):
    def test_two_runtime_files_restore_the_complete_old_source_bytes(self):
        self.assertEqual(len(SCOPE["records"]), 2)
        for record in SCOPE["records"]:
            self.assertEqual(hashlib.sha256(restore(record).encode()).hexdigest(), record["before_sha256"])

    def test_manifest_wire_request_response_and_draw_are_unchanged(self):
        pin = json.loads((HERE / "modifier-anchor-oracle.json").read_text())
        targets = {"horizon-mixin.toml", "source-materials-book-wire.rs", "source-materials-book-request.rs",
                   "source-materials-book-response.rs", "source-materials-book-draw.rs", "source-materials-book-identity.rs"}
        seen = set()
        for source in pin["source_hashes"]:
            if Path(source["path"]).name in targets:
                seen.add(Path(source["path"]).name)
                self.assertEqual(hashlib.sha256((ROOT / source["path"]).read_bytes()).hexdigest(), source["sha256"])
        self.assertEqual(seen, targets)

    def test_existing_font_call_lines_keep_the_same_order_and_arguments(self):
        record = SCOPE["records"][0]
        old = restore(record)
        new = (ROOT / record["path"]).read_text()
        def font_lines(source):
            return [line.strip() for line in source.splitlines() if any(
                token in line for token in ["font.", "split_styled(", "split_page(", "split_page_width("])]
        self.assertEqual(font_lines(new), font_lines(old))
        self.assertIn('if !text.is_empty() {\n                let parts = remaining.unwrap_or_else(', new)
        self.assertLess(new.index('let entry_start = chapter_pages.len();'), new.index('anchors.record(relative,'))
        self.assertLess(new.index('                    chapter_pages.push(page);'), new.index('anchors.record(relative,'))
        self.assertIn('chapter_pages.get(entry_start)\n                    .and_then(|page| page.spans.first())', new)

    def test_record_admission_paths_and_atomic_shift_remain_narrow(self):
        source = (MOD / "source-materials-book-index.rs").read_text()
        helper = source.split('// Only the fixed modifier overview entries', 1)[1].split('#[cfg(test)]', 1)[0]
        for path in ['index', 'traits', 'upgrades', 'slotless', 'abilities', 'defense']:
            self.assertIn(f'"modifiers/{path}.json" => Some(', helper)
        self.assertEqual(helper.count('=> Some('), 6)
        self.assertIn('title.is_empty() || first != Some(title)', helper)
        self.assertIn('self.0[slot].get_or_insert(offset)', helper)
        self.assertLess(helper.index('let mut shifted = self.0;'), helper.index('offset.checked_add(count)?'))
        self.assertLess(helper.index('offset.checked_add(count)?'), helper.index('self.0 = shifted;'))
        for token in ['Game', 'Text', 'Font', 'HashMap', 'Vec<', 'Mutex', 'Atomic', 'send(', 'open_book']:
            self.assertNotIn(token, helper)

    def test_final_links_keep_source_reads_label_admission_and_checked_destination(self):
        source = (MOD / "source-materials-book.rs").read_text()
        self.assertIn('key == "modifiers" && labels.contains_key("modifiers")', source)
        self.assertIn('read(path, &format!("{root}en_us/modifiers/index.json"))', source)
        self.assertIn('read(path, &format!("{root}en_us/modifiers/{name}.json"))', source)
        self.assertIn('first_page.checked_add(i32::try_from(offset).ok()?)?', source)
        self.assertIn('anchors.prepend(chapter_pages.len().checked_sub(before_notices)?)?', source)
        self.assertNotIn('labels.get("modifiers").is_some_and(|label| *label == heading)', source)

    def test_five_native_rust_cases_are_present_not_claimed_executed(self):
        source = (MOD / "source-materials-book-index.rs").read_text()
        tests = source.split('mod modifier_anchor_tests {', 1)[1].split('\n#[cfg(test)]\nmod tests', 1)[0]
        self.assertEqual(tests.count('#[test]'), 5)
        self.assertIn('usize::MAX', tests)
        self.assertIn('None);', tests)


if __name__ == "__main__":
    unittest.main()
