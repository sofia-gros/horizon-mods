"""Source-only checks. No Cargo, patch application, generated fixture or native run."""
import argparse
import difflib
import hashlib
import json
from pathlib import Path
import re
import unittest
import zipfile

HERE = Path(__file__).resolve().parent
COMMON = LATEST = ARCHIVE = None

def sha(data): return hashlib.sha256(data).hexdigest()

class SourceChecks(unittest.TestCase):
    def test_frozen_candidate_sources_and_status(self):
        meta = json.loads((HERE / 'inputs.json').read_text())
        self.assertEqual(meta['status'], 'source_prepared_rust_unrun_patch_unapplied')
        self.assertEqual({p.name for p in HERE.iterdir()}, set(meta['source_sha256']) | {'inputs.json'})
        for name, digest in meta['source_sha256'].items():
            self.assertFalse((HERE / name).is_symlink())
            self.assertEqual(sha((HERE / name).read_bytes()), digest)

    def test_six_cases_and_actual_native_include(self):
        meta = json.loads((HERE / 'inputs.json').read_text())
        source = (HERE / 'template_tests.rs').read_text()
        self.assertEqual(re.findall(r'#\[test\]\s*fn ([a-z0-9_]+)', source), meta['expected_tests'])
        self.assertEqual(len(meta['expected_tests']), 6)
        self.assertIn('include!("native-template.rs")', source)
        self.assertNotIn('impl Template', source)
        self.assertNotIn('fn over(', source)

    def test_latest_actual_fragment_identity(self):
        meta = json.loads((HERE / 'inputs.json').read_text())
        record = meta['native_template']
        source = (LATEST / record['path']).read_text()
        self.assertEqual(source.count(record['start']), 1)
        self.assertEqual(source.count(record['end']), 1)
        actual = source[source.index(record['start']):source.index(record['end'])]
        self.assertEqual(sha(actual.encode()), record['sha256'])
        self.assertIn('pub components: Option<Tag>', actual)
        self.assertIn('fn over(', actual)
        self.assertIn('fn assembly_components(', actual)

    def test_two_line_patch_reproduces_only_exact_factory_changes(self):
        records = json.loads((HERE / 'fixture-records.json').read_text())
        self.assertEqual(len(records), 2)
        patch = []
        for row in records:
            old = (COMMON / row['path']).read_text()
            self.assertEqual(sha(old.encode()), row['old_sha256'])
            self.assertEqual(old.count(row['after']), 1)
            new = old.replace(row['after'], row['after'] + '            components: None,\n', 1)
            self.assertEqual(sha(new.encode()), row['proposed_sha256'])
            self.assertNotIn('.over(', old)
            patch.extend(difflib.unified_diff(old.splitlines(True), new.splitlines(True), fromfile='a/'+row['path'], tofile='b/'+row['path']))
        self.assertEqual(''.join(patch), (HERE / 'fixture-literals.patch').read_text())

    def test_actual_common_archive_scope(self):
        audit = json.loads((HERE / 'official-audit.json').read_text())
        self.assertEqual(sha(ARCHIVE.read_bytes()), audit['common_archive_sha256'])
        with zipfile.ZipFile(ARCHIVE) as archive:
            self.assertEqual(len(archive.namelist()), audit['common_archive_members'])
            references = []
            for name in archive.namelist():
                if not name.endswith('.rs'): continue
                text = archive.read(name).decode()
                self.assertIsNone(re.search(r'\bTemplate\s*\{', text), name)
                if 'Template' in text: references.append(name)
            self.assertEqual(len(references), 2)
            self.assertTrue(any(p.endswith('/crafting-match.rs') for p in references))
            self.assertTrue(any(p.endswith('/crafting/matching.rs') for p in references))

    def test_latest_source_hashes_match_read_only_audit(self):
        audit = json.loads((HERE / 'official-audit.json').read_text())
        for row in audit['files']:
            self.assertEqual(sha((LATEST / row['path']).read_bytes()), row['new_sha256'])
        for row in audit['regions']:
            text = (LATEST / row['path']).read_text()
            actual = text[text.index(row['start']):text.index(row['end'])]
            self.assertEqual(sha(actual.encode()), row['new_sha256'])
        same = {r['path'] for r in audit['files'] if r['same']}
        self.assertIn('crates/horizon-client/src/item_sync.rs', same)
        self.assertIn('crates/horizon-world/src/item_defaults.rs', same)
        self.assertIn('crates/horizon-client/src/recipe_book.rs', same)

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--common', type=Path, required=True)
    parser.add_argument('--latest-sources', type=Path, required=True)
    parser.add_argument('--archive', type=Path, required=True)
    args, rest = parser.parse_known_args()
    COMMON, LATEST, ARCHIVE = args.common, args.latest_sources, args.archive
    unittest.main(argv=[__file__, *rest])
