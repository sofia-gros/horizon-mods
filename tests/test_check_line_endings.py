import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('check_line_endings', ROOT / 'tools/check_line_endings.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

RULE = module.REQUIRED_RULE + '\n'


class LineEndingChecks(unittest.TestCase):
    def test_clean_listing_passes(self):
        listing = 'i/lf    w/lf    attr/text eol=lf \ta.py\ni/-text w/-text attr/binary        \tb.png\n'
        self.assertEqual(module.violations(listing, RULE), [])

    def test_crlf_in_the_index_is_rejected(self):
        listing = 'i/crlf  w/crlf  attr/text eol=lf \ta.py\n'
        self.assertEqual(len(module.violations(listing, RULE)), 1)

    def test_crlf_checkout_of_an_lf_file_is_reported_with_the_repair(self):
        found = module.violations('i/lf    w/crlf  attr/text eol=lf \ta.py\n', RULE)
        self.assertEqual(len(found), 1)
        self.assertIn('git reset', found[0])

    def test_batch_files_may_be_crlf(self):
        self.assertEqual(module.violations('i/crlf  w/crlf  attr/text eol=crlf \trun.cmd\n', RULE), [])

    def test_missing_attributes_rule_is_rejected(self):
        self.assertEqual(len(module.violations('i/lf    w/lf    attr/ \ta.py\n', '')), 1)


if __name__ == '__main__':
    unittest.main()
