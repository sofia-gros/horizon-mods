import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

PATH = Path(__file__).resolve().parents[1] / 'tools/prepare_language_json.py'
SPEC = importlib.util.spec_from_file_location('prepare_language_json', PATH)
prepare = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(prepare)


class LanguagePreparation(unittest.TestCase):
    def test_exact_unicode_key_namespace_and_format_spelling(self):
        data = '"fluid.meadow.rain" = "雨𐐷 %d / %1$s / %%"\n"bare"="plain"\n'
        self.assertEqual(json.loads(prepare.convert_toml(data.encode())),
                         {'fluid.meadow.rain': '雨𐐷 %d / %1$s / %%', 'bare': 'plain'})

    def test_dotted_unquoted_keys_are_not_silently_flattened(self):
        with self.assertRaises(ValueError): prepare.convert_toml(b'fluid.meadow.rain="water"')

    def test_duplicate_keys_are_rejected(self):
        with self.assertRaises(ValueError): prepare.convert_toml(b'"k"="one"\n"k"="two"')

    def test_nonstring_values_reject_without_coercion(self):
        for value in ['1', 'true', '1.5', '["a"]', '1979-05-27', '{a="b"}']:
            with self.subTest(value=value), self.assertRaises(ValueError):
                prepare.convert_toml(f'k={value}'.encode())

    def test_invalid_utf8_rejects(self):
        with self.assertRaises(UnicodeError): prepare.convert_toml(b'k="\xff"')

    def test_input_and_entry_limits_reject(self):
        with self.assertRaises(ValueError): prepare.convert_toml(b'k="v"', max_bytes=4)
        with self.assertRaises(ValueError): prepare.convert_toml(b'a="x"\nb="y"', max_entries=1)

    def test_encoded_output_limit_rejects(self):
        with self.assertRaises(ValueError): prepare.convert_toml(b'a="x"', max_bytes=5)

    def test_existing_destination_and_source_are_preserved(self):
        with tempfile.TemporaryDirectory() as tmp:
            source, destination = Path(tmp)/'en_us.toml', Path(tmp)/'en_us.json'
            source.write_bytes(b'k="v"'); destination.write_bytes(b'existing bytes')
            with self.assertRaises(FileExistsError): prepare.prepare(source, destination)
            self.assertEqual(source.read_bytes(), b'k="v"')
            self.assertEqual(destination.read_bytes(), b'existing bytes')
            self.assertEqual(sorted(p.name for p in Path(tmp).iterdir()), ['en_us.json','en_us.toml'])

    def test_success_only_creates_requested_json_and_repeated_call_conflicts(self):
        with tempfile.TemporaryDirectory() as tmp:
            source, destination = Path(tmp)/'ja_jp.toml', Path(tmp)/'ja_jp.json'
            source.write_text('"fluid.meadow.rain"="雨"',encoding='utf-8')
            prepare.prepare(source,destination)
            original = destination.read_bytes()
            self.assertEqual(json.loads(original),{'fluid.meadow.rain':'雨'})
            with self.assertRaises(FileExistsError): prepare.prepare(source,destination)
            self.assertEqual(destination.read_bytes(),original)

    def test_invalid_input_creates_no_destination(self):
        with tempfile.TemporaryDirectory() as tmp:
            source,destination=Path(tmp)/'en_us.toml',Path(tmp)/'en_us.json'
            source.write_text('a=1')
            with self.assertRaises(ValueError): prepare.prepare(source,destination)
            self.assertFalse(destination.exists())
            self.assertEqual(list(Path(tmp).iterdir()),[source])

    def test_symlink_destination_is_not_followed_or_replaced(self):
        with tempfile.TemporaryDirectory() as tmp:
            source,destination,target=Path(tmp)/'en_us.toml',Path(tmp)/'en_us.json',Path(tmp)/'target'
            source.write_text('k="v"'); target.write_bytes(b'keep'); destination.symlink_to(target)
            with self.assertRaises(FileExistsError): prepare.prepare(source,destination)
            self.assertTrue(destination.is_symlink()); self.assertEqual(target.read_bytes(),b'keep')


if __name__ == '__main__': unittest.main()
