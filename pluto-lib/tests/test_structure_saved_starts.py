"""Execute the production natural-start adapter with a trace-only host double.

Checks saved-source routing and regeneration decisions, not native decoding or
world-generation parity. Native decoder semantics remain the pinned host's.
"""
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class StructureSavedStartsTests(unittest.TestCase):
    def test_saved_source_precedes_generation(self):
        fixture = (ROOT / 'pluto-lib/tests/fixtures/structure_saved_starts.rs').read_text()
        helper = (ROOT / 'pluto-lib/mod/pluto-structure-natural.rs').as_posix()
        with tempfile.TemporaryDirectory(prefix='structure-saved-starts-') as temp:
            source = Path(temp) / 'saved.rs'
            binary = Path(temp) / 'saved.exe'
            source.write_text(newline='\n', data=fixture.replace('PRODUCTION_HELPER', '"' + helper + '"'), encoding='utf-8')
            subprocess.run(['rustc', '+1.95.0', '--edition=2021', '--test', str(source), '-o', str(binary)], check=True)
            subprocess.run([str(binary)], check=True)


if __name__ == '__main__':
    unittest.main()
