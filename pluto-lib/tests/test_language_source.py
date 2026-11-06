from pathlib import Path
import tomllib
import unittest

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
        for entry in parsed.get('inject',[])+parsed.get('state',[]):
            self.assertNotIn('pluto_language',str(entry))
            self.assertNotIn('language.rs',str(entry))

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
