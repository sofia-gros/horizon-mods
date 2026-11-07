import hashlib
from pathlib import Path
import tomllib
import unittest
from unittest.mock import patch

ROOT=Path(__file__).resolve().parents[1]
MODULE=ROOT/'mixins/pluto_native_creative/language.rs'
MANIFEST=ROOT/'mixins/pluto_native_creative/horizon-mixin.toml'
ENTRY='''\n[[module]]
id = "language"
file = "crates/horizon-client/src/client_lib.rs"
name = "pluto_language"
source = "language.rs"
visibility = "pub"
'''


class LanguageSource(unittest.TestCase):
    def test_exact_module_entry_without_language_hooks_or_states(self):
        parsed=tomllib.loads(MANIFEST.read_text())
        entries=[m for m in parsed['module'] if m.get('id')=='language' or m.get('name')=='pluto_language' or m.get('source')=='language.rs']
        self.assertEqual(entries,tomllib.loads(ENTRY)['module'])
        for entry in parsed.get('injection',[])+parsed.get('state',[]):
            self.assertNotIn('pluto_language',str(entry))
            self.assertNotIn('language.rs',str(entry))

    def test_guard_rejects_language_injections_and_states(self):
        manifest = MANIFEST.read_text()
        cases = [
            ('injection', 'target = "horizon_client::pluto_language::text"'),
            ('injection', 'helper = "language.rs"'),
            ('state', 'ty = "crate::pluto_language::Key"'),
            ('state', 'default = \'include_str!("language.rs")\''),
        ]
        for table, field in cases:
            with self.subTest(table=table, field=field):
                mutated = manifest + f'\n[[{table}]]\nid = "unexpected-language-owner"\n{field}\n'
                with patch.object(Path, 'read_text', return_value=mutated):
                    with self.assertRaises(AssertionError):
                        self.test_exact_module_entry_without_language_hooks_or_states()

    def test_composition_changes_only_native_text_import_and_exact_key_reexport(self):
        source=MODULE.read_text()
        self.assertEqual(source.count('pub use crate::pluto_language_text::{Key, key, Text};'),1)
        original=source.replace('use horizon_client_core::lang;\npub use crate::pluto_language_text::{Key, key, Text};',
                                'use horizon_client_core::{lang, text::Text};')
        self.assertEqual(hashlib.sha256(original.encode()).hexdigest(),
                         'f587b3e7dab7da1374778b84cbd75fbe39f4b8ca664c1157e198b5c53eb2972a')
        parsed=tomllib.loads(MANIFEST.read_text())
        expected={'id':'language-text','file':'crates/horizon-client/src/client_lib.rs',
                  'name':'pluto_language_text','source':'language-text.rs','visibility':'pub'}
        self.assertEqual([m for m in parsed['module'] if m.get('id')=='language-text' or m.get('name')=='pluto_language_text' or m.get('source')=='language-text.rs'],[expected])

    def test_current_owner_and_native_parsing_are_delegated(self):
        body=MODULE.read_text().split('#[cfg(test)]')[0]
        self.assertNotIn('//!',body)
        self.assertNotIn('#![',body)
        for delegation in ['lang::Language::load(sources, code)','lang::get(key)',
                           'lang::get_or(key, fallback)','lang::has(key)','lang::code()',
                           'lang::tr(key, args)','Text::translate(key, args)',
                           'Text::translate_with_fallback(key, fallback, args)']:
            self.assertEqual(body.count(delegation),1)
        for forbidden in ['static ', 'OnceLock', 'RwLock', 'HashMap', 'lang::set(', 'serde_json', 'toml::']:
            self.assertNotIn(forbidden,body)

    def test_native_cases_are_actual_module_tests_not_copied_types(self):
        body=MODULE.read_text()
        self.assertEqual(body.count('#[test]'),8)
        self.assertIn('impl lang::ResourceSource for Source',body)
        self.assertIn('lang::scoped(',body)
        self.assertNotIn('struct Language',body)

    def test_preparation_does_not_patch_runtime_format_or_locale(self):
        body=(ROOT/'tools/prepare_language_json.py').read_text()
        self.assertIn('tomllib.loads(data.decode',body)
        self.assertIn('os.link(temporary, destination)',body)
        self.assertNotIn('replace(',body)
        self.assertNotIn('lang::set',body)


if __name__=='__main__': unittest.main()
