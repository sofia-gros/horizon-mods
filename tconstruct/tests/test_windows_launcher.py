"""Check the Windows launcher keeps one selected interpreter throughout preparation."""
from pathlib import Path
import unittest


class WindowsLauncher(unittest.TestCase):
    def test_preparation_uses_selected_interpreter(self):
        script = (Path(__file__).resolve().parents[1] / 'tools/play-windows.ps1').read_text()
        self.assertIn('$python = Find-Python', script)
        for tool in ('stage.py', 'resources.py', 'prepare_horizon_source_data.py', 'prepare_bundled_forge.py'):
            self.assertIn("Run $python @('tools/" + tool + "'", script)
        self.assertNotIn('Run py ', script)
        self.assertIn("$mise.Source exec -- python -c", script)
        self.assertIn('sys.version_info >= (3, 12)', script)


if __name__ == '__main__':
    unittest.main()
