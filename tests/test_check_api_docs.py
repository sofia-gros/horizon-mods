import importlib.util
import tempfile
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('check_api_docs', ROOT / 'tools/check_api_docs.py')
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)

SOURCE = '''
/// A thing.
pub struct Thing {
    pub size: u32,
}

impl Thing {
    pub fn grow(&self) {}
    fn hidden(&self) {}
}

pub(crate) fn internal() {}
pub fn free() {}
'''

PAGE = '''# Page

### `Thing`

Has a size.

```rust
let _ = 1;
```

### `Thing::grow`

```rust
let _ = 2;
```

### `free`

No example here.
'''


class ApiDocsCheck(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        base = Path(self.temp.name)
        (base / 'src').mkdir()
        (base / 'pages').mkdir()
        (base / 'src/thing.rs').write_text(SOURCE, encoding='utf-8', newline='\n')
        (base / 'pages/page.md').write_text(PAGE, encoding='utf-8', newline='\n')
        checker.ROOT, checker.SOURCE, checker.PAGES = base, base / 'src', base / 'pages'

    def tearDown(self):
        self.temp.cleanup()

    def test_only_public_items_are_required_and_a_missing_example_or_field_is_reported(self):
        found = checker.problems()
        self.assertEqual(len(found), 1, found)
        self.assertIn('`free`', found[0])
        self.assertIn('rust example', found[0])

    def test_an_undocumented_public_function_and_a_stale_section_are_reported(self):
        (checker.SOURCE / 'thing.rs').write_text(SOURCE + 'pub fn added() {}\n', encoding='utf-8', newline='\n')
        (checker.PAGES / 'page.md').write_text(PAGE + '\n### `gone`\n\n```rust\nlet _ = 3;\n```\n', encoding='utf-8', newline='\n')
        found = '\n'.join(checker.problems())
        self.assertIn('`added` has no section', found)
        self.assertIn('section `gone` documents nothing public', found)

    def test_a_public_field_must_be_named_in_its_struct_section(self):
        (checker.PAGES / 'page.md').write_text(PAGE.replace('Has a size.', 'Has something.'), encoding='utf-8', newline='\n')
        self.assertTrue(any('Thing.size' in line for line in checker.problems()))


if __name__ == '__main__':
    unittest.main()
