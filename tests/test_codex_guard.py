import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
import codex_guard as guard  # noqa: E402

TASK = {
    'id': 'world-test',
    'allowed_paths': ['tconstruct/mixins/tconstruct/api/src/**', 'docs/inventory/world.json', 'tools/injection_limits.json'],
    'max_files': 3,
    'max_changed_lines': 100,
    'inventory': {'file': 'docs/inventory/world.json', 'classes': ['block/SlimeGrassBlock.java']},
}


def change(path, status='M', added=10, removed=0, crlf=False):
    return {'path': path, 'status': status, 'added': added, 'removed': removed, 'crlf': crlf}


class CodexGuard(unittest.TestCase):
    def test_a_change_inside_the_task_passes(self):
        self.assertEqual(guard.violations(TASK, [change('tconstruct/mixins/tconstruct/api/src/world.rs')]), [])

    def test_a_path_outside_the_task_and_a_forbidden_one_are_caught(self):
        found = guard.violations(TASK, [change('pluto-lib/mod/api/src/hooks.rs'), change('AGENTS.md')])
        self.assertEqual(len(found), 2)
        self.assertIn('allowed_paths', found[0])
        self.assertIn('off limits', found[1])

    def test_a_task_may_open_one_forbidden_file_by_name(self):
        self.assertEqual(guard.violations(TASK, [change('tools/injection_limits.json')]), [])

    def test_new_tests_markdown_and_binaries_are_caught_unless_asked_for(self):
        task = dict(TASK, allowed_paths=['**'])
        paths = ['tconstruct/mixins/tconstruct/api/src/tests/a.rs', 'notes.md', 'x.png']
        found = guard.violations(task, [change(p, 'A') for p in paths])
        self.assertEqual(len(found), 3)
        self.assertEqual(guard.violations(dict(task, tests_allowed=True, markdown_allowed=True, binary_allowed=True), [change(p, 'A') for p in paths]), [])

    def test_crlf_size_and_deletion_limits(self):
        found = guard.violations(TASK, [change('tconstruct/mixins/tconstruct/api/src/a.rs', crlf=True), change('tconstruct/mixins/tconstruct/api/src/b.rs', 'D')])
        self.assertTrue(any('CRLF' in f for f in found) and any('deleted' in f for f in found))
        big = [change(f'tconstruct/mixins/tconstruct/api/src/f{i}.rs', added=60) for i in range(2)]
        self.assertTrue(any('lines changed' in f for f in guard.violations(TASK, big)))

    def test_the_inventory_may_only_change_for_named_classes_and_never_to_ported(self):
        before = {'classes': {'block/SlimeGrassBlock.java': {'status': 'todo', 'note': 'x'}, 'block/SlimeDirtBlock.java': {'status': 'todo', 'note': 'x'}}}
        ok = {'classes': {**before['classes'], 'block/SlimeGrassBlock.java': {'status': 'ported-unchecked', 'note': 'World block behavior implemented; probe checks growth [world-test]'}}}
        self.assertEqual(guard.violations(TASK, [], before, ok), [])
        ported = {'classes': {**before['classes'], 'block/SlimeGrassBlock.java': {'status': 'ported', 'note': 'done'}}}
        self.assertTrue(guard.violations(TASK, [], before, ported))
        other = {'classes': {**before['classes'], 'block/SlimeDirtBlock.java': {'status': 'ported-unchecked', 'note': 'World block behavior implemented; probe checks growth [world-test]'}}}
        self.assertTrue(guard.violations(TASK, [], before, other))


if __name__ == '__main__':
    unittest.main()
