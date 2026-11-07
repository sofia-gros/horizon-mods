import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('check_docs_budget', ROOT / 'tools/check_docs_budget.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class DocsBudget(unittest.TestCase):
    def test_listed_documents_and_notices_pass(self):
        self.assertEqual(module.violations(['README.md', 'docs/roadmap.md', 'tconstruct/third_party/x-NOTICE.md']), [])

    def test_a_new_per_implementation_note_is_rejected(self):
        found = module.violations(['pluto-lib/docs/some-feature-validation.md'])
        self.assertEqual(len(found), 1)
        self.assertIn('allow list', found[0])

    def test_an_unreadably_long_document_is_rejected(self):
        self.assertEqual(len(module.violations(['docs/roadmap.md'], {'docs/roadmap.md': 200_000})), 1)


if __name__ == '__main__':
    unittest.main()
