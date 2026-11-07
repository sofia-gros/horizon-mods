import json, subprocess, sys, tempfile, unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
import lint_forbidden_api as lint


class ForbiddenApiLint(unittest.TestCase):
    def run_in_repo(self, files, baseline=None):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            subprocess.run(['git', 'init', '-q'], cwd=root, check=True)
            for name, text in files.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(text, encoding='utf-8')
            subprocess.run(['git', 'add', '-A'], cwd=root, check=True)
            lint.ROOT, lint.BASELINE = root, root / 'baseline.json'
            if baseline is not None:
                lint.BASELINE.write_text(json.dumps(baseline))
            return lint.main([])

    def test_clean_tree_passes(self):
        self.assertEqual(self.run_in_repo({'tconstruct/mixins/tconstruct/a.rs': 'fn ok() {}'}), 0)

    def test_fork_reference_is_zero_tolerance(self):
        self.assertEqual(self.run_in_repo({'x/Cargo.toml': 'a = { git = "https://github.com/sofia-gros/horizon.git" }'}), 1)

    def test_fork_named_like_mods_repo_is_not_flagged(self):
        self.assertEqual(self.run_in_repo({'README.md': 'github.com/sofia-gros/horizon-mods'}), 0)

    def test_non_official_git_dependency_fails(self):
        self.assertEqual(self.run_in_repo({'p/Cargo.toml': 'a = { git = "https://example.com/x.git" }'}), 1)

    def test_legacy_host_use_tolerated_but_growth_fails(self):
        f = 'tconstruct/mixins/tconstruct/a.rs'
        base = {'host-crate-direct': {f: 1}}
        self.assertEqual(self.run_in_repo({f: 'use horizon_world::A;'}, base), 0)
        self.assertEqual(self.run_in_repo({f: 'use horizon_world::A; use horizon_items::B;'}, base), 1)

    def test_new_file_starts_at_zero(self):
        self.assertEqual(self.run_in_repo({'tconstruct/mixins/tconstruct/new.rs': 'game.inventory.clear();'}, {}), 1)


if __name__ == '__main__':
    unittest.main()
