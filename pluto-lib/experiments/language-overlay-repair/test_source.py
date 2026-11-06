"""Pure source-contract tests, not execution of the Rust merge implementation."""
import ast
import importlib.util
import json
import os
from pathlib import Path
import re
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('repair_source', HERE / 'source.py')
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
OFFICIAL = Path(os.environ['PLUTO_SELECTED_SOURCE'])
CONFIG = json.loads((HERE / 'inputs.json').read_bytes())
HELPER = (HERE / 'merge.rs').read_bytes()


class SourceTests(unittest.TestCase):
    def test_exact_seven_official_sources_and_helper(self):
        self.assertEqual(len(CONFIG['official_sources']), 7)
        for p, h in CONFIG['official_sources'].items(): m.read(OFFICIAL, p, h)
        self.assertEqual(m.sha(HELPER), CONFIG['helper_sha256'])

    def test_native_admission_and_build_clash_are_both_required(self):
        native = (OFFICIAL / 'crates/horizon-mods/src/content.rs').read_bytes()
        a = CONFIG['admission_contract']; start = native.index(a['start'].encode()); end = native.index(a['end'].encode())
        contract = native[start:end]
        self.assertEqual(m.sha(contract), a['sha256'])
        self.assertIn(b'let owned = own || manifest.namespaces.iter().any(|n| n == namespace);', contract)
        self.assertIn(b'let allowed = owned || (top == "data" && shared && rest.starts_with("tags/")) || atlas;', contract)
        writer = (OFFICIAL / m.TARGET).read_bytes()
        build = writer.index(b'pub fn build(')
        self.assertLess(writer.index(b'horizon_mods::content::clash(b, a)', build), writer.index(b'Overlay::new(base, out)?', build))

    def test_complete_native_inverse_and_deterministic_source(self):
        first, evidence = m.compose(OFFICIAL)
        second, again = m.compose(OFFICIAL)
        self.assertEqual((first, evidence), (second, again))
        self.assertEqual(m.inverse(first, HELPER), (OFFICIAL / m.TARGET).read_bytes())
        self.assertEqual(evidence['status'], 'source_composed_only')
        self.assertFalse(evidence['native_executed'])
        self.assertFalse(evidence['mixins_placement_verified'])
        self.assertFalse(evidence['production_activated'])

    def test_native_package_read_before_candidate_then_original_move(self):
        candidate, _ = m.compose(OFFICIAL)
        reader = candidate.index(b'let bytes = reader.read(file)?;', candidate.index(b'pub fn build('))
        prepare = candidate.index(b'let pluto_language_bytes =')
        write = candidate.index(b'overlay.write(file, &pluto_language_bytes)?;')
        following = candidate.index(b'if biomes::is_modifier(file)', write)
        moved = candidate.index(b'let text = String::from_utf8(bytes)', following)
        self.assertLess(reader, prepare)
        self.assertLess(prepare, write)
        self.assertLess(write, following)
        self.assertLess(following, moved)
        self.assertIn(b'                }\n                if biomes::is_modifier(file)', candidate)

    def test_native_overlay_write_and_resource_root_are_byte_identical(self):
        original = (OFFICIAL / m.TARGET).read_bytes()
        candidate, _ = m.compose(OFFICIAL)
        for start, end in [(b'    fn write(&self, rel: &str, bytes: &[u8])', b'\nfn out_path('),
                           (b'pub fn resource_root(', b'\n/// The length of a root')]:
            at, until = original.index(start), original.index(end, original.index(start))
            self.assertIn(original[at:until], candidate)

    def test_stamp_tag_is_inside_original_hash_sequence_not_a_suffix(self):
        candidate, _ = m.compose(OFFICIAL)
        at = candidate.index(b'fn stamp(')
        stamp = candidate[at:candidate.index(b'\n#[cfg(test)]', at)]
        self.assertLess(stamp.index(m.STAMP_NEW), stamp.index(b'parts.push(meta(&std::env::current_exe()?)?);'))
        self.assertLess(stamp.index(b'POLICY_ID'), stamp.index(b'let mut hasher'))
        self.assertIn(b'hasher.update((part.len() as u64).to_le_bytes());', stamp)
        self.assertIn(b'Ok(format!("{:x}", hasher.finalize()))', stamp)

    def test_ambiguous_or_already_applied_native_source_rejects(self):
        original = (OFFICIAL / m.TARGET).read_bytes()
        for changed in [original + m.WRITER_OLD, original.replace(m.WRITER_OLD, b''),
                        original + m.STAMP_OLD, original + b'pluto_language_overlay']:
            with self.assertRaises(ValueError): m.candidate(changed, HELPER)
        candidate, _ = m.compose(OFFICIAL)
        with self.assertRaises(ValueError): m.inverse(candidate.replace(m.WRITER_NEW, b'', 1), HELPER)
        with self.assertRaises(ValueError): m.inverse(candidate + b'\n', HELPER)

    def test_modified_source_or_helper_hash_rejects(self):
        with self.assertRaises(ValueError): m.read(OFFICIAL, m.TARGET, '0' * 64)
        with self.assertRaises(ValueError): m.read(HERE, 'merge.rs', '0' * 64)

    def test_selected_input_parent_links_and_escape_reject(self):
        with tempfile.TemporaryDirectory(prefix='pluto-repair-source-') as tmp:
            root = Path(tmp); (root / 'real').mkdir(); (root / 'real' / 'x').write_bytes(b'x')
            (root / 'alias').symlink_to(root / 'real', target_is_directory=True)
            with self.assertRaises(ValueError): m.read(root, 'alias/x', m.sha(b'x'))
            with self.assertRaises(ValueError): m.read(root / 'alias', 'x', m.sha(b'x'))
            with self.assertRaises(ValueError): m.read(root, '../x', m.sha(b'x'))
            with self.assertRaises(ValueError): m.read(root, '/x', m.sha(b'x'))

    def test_all_fourteen_rust_cases_prepared_not_ignored(self):
        names = [v.decode() for v in re.findall(rb'#\[test\]\s*fn\s+(\w+)\(', HELPER)]
        self.assertEqual(names, CONFIG['prepared_tests'])
        self.assertEqual(len(names), len(set(names)))
        self.assertEqual(len(names), 14)
        self.assertNotIn(b'#[ignore]', HELPER)

    def test_declared_bounds_and_strict_duplicate_rejection_are_present(self):
        for fragment in [b'MAX_INPUT_BYTES: usize = 1024 * 1024', b'MAX_ENTRIES: usize = 16_384',
                         b'MAX_DECODED_BYTES: usize = 2 * MAX_INPUT_BYTES', b'MAX_OUTPUT_BYTES: usize = 2 * MAX_INPUT_BYTES',
                         b'map.contains_key(&key)', b'a.next_value::<String>()?', b'd.deserialize_map(Flat)',
                         b'serde_json::from_slice::<Strings>(bytes)', b'lower.keys().all(|key| higher.contains_key(key))',
                         b'result.extend(higher)', b'merged.map_or(Cow::Borrowed(higher), Cow::Owned)']:
            self.assertIn(fragment, HELPER)

    def test_only_prepared_base_read_no_native_or_locale_owner(self):
        production = HELPER.split(b'#[cfg(test)]')[0]
        self.assertIn(b'lower_bytes(&base.join(rel))', production)
        self.assertNotIn(b'fs::write', production)
        self.assertNotIn(b'remove_file', production)
        self.assertNotIn(b'Language::', production)
        self.assertNotIn(b'read_lang_file', production)
        self.assertNotIn(b'OnceLock', production)
        self.assertNotIn(b'Mutex', production)

    def test_composer_has_no_execution_or_output_paths(self):
        tree = ast.parse((HERE / 'source.py').read_text())
        imports = {n.module for n in ast.walk(tree) if isinstance(n, ast.ImportFrom)}
        imports |= {a.name for n in ast.walk(tree) if isinstance(n, ast.Import) for a in n.names}
        self.assertFalse(imports & {'subprocess', 'shutil', 'os'})
        calls = {n.func.attr for n in ast.walk(tree) if isinstance(n, ast.Call) and isinstance(n.func, ast.Attribute)}
        self.assertFalse(calls & {'write_bytes', 'write_text', 'mkdir', 'unlink', 'rename', 'run', 'Popen'})
        for call in ast.walk(tree):
            if isinstance(call, ast.Call) and isinstance(call.func, ast.Attribute) and call.func.attr == 'open':
                self.assertEqual(ast.literal_eval(call.args[0]), 'rb')


if __name__ == '__main__': unittest.main()
