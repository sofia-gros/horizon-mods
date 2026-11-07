"""Current ordinary callers, not archived experimental CI. No native execution."""
import ast
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import textwrap
import types
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
INVERSE = ROOT / 'tconstruct/experiments/windows-stage-package-contract/callers-inverse.json'
WORKFLOW = ROOT / '.github/workflows/mixins.yml'
FLAGS = ('--tconstruct-archive', '--pluto-archive', '--tconstruct-archive-sha256', '--pluto-archive-sha256')


def workflow_python():
    source = WORKFLOW.read_text(encoding='utf-8')
    start = "          python3 - <<'PY_STAGE'\n"; end = '          PY_STAGE\n'
    if source.count(start) != 1 or source.count(end) != 1:
        raise ValueError('Expected exactly one ordinary package/stage caller')
    return textwrap.dedent(source.split(start, 1)[1].split(end, 1)[0])


class StageCallers(unittest.TestCase):
    def test_exact_two_caller_inverse_preserves_all_other_workflow_and_fixture_code(self):
        records = json.loads(INVERSE.read_bytes())['files']
        self.assertEqual(set(records), {'.github/workflows/mixins.yml', 'tconstruct/tests/test_stage.py'})
        for relative, row in records.items():
            raw = (ROOT / relative).read_bytes()
            self.assertEqual(hashlib.sha256(raw).hexdigest(), row['after_sha256'])
            text = raw.decode('utf-8')
            for edit in reversed(row['edits']):
                if isinstance(edit, dict):
                    before, after = edit['replace_all']
                    self.assertEqual(text.count(after), edit['count'])
                    text = text.replace(after, before)
                else:
                    before, after = edit
                    self.assertEqual(text.count(after), 1)
                    text = text.replace(after, before, 1)
            self.assertEqual(hashlib.sha256(text.encode('utf-8')).hexdigest(), row['before_sha256'])

    def test_workflow_packages_verifies_then_passes_same_receipt_and_members(self):
        events = []; tc = Path('/fixture/packages/tc-mods'); pl = Path('/fixture/packages/pluto-mods/pluto_native_creative')
        receipt = {'packages': [{'archive_sha256': '1' * 64}, {'archive_sha256': '2' * 64}]}
        def prepare(*args):
            events.append(('prepare', args)); return tc, pl, receipt
        def verify(*args):
            events.append(('verify', args))
        module = types.SimpleNamespace(prepare=prepare, verify_prepared=verify)
        with patch.dict(sys.modules, prepare_playtest_bundles=module), patch.dict(os.environ, RUNNER_TEMP='/fixture'), \
             patch.object(subprocess, 'check_output', return_value='a' * 40 + '\n') as head, \
             patch.object(subprocess, 'run', side_effect=lambda argv, **kw: events.append(('stage', argv, kw))):
            exec(compile(workflow_python(), str(WORKFLOW), 'exec'), {})
        self.assertEqual([x[0] for x in events], ['prepare', 'verify', 'stage'])
        self.assertEqual(events[1][1], (tc, pl, receipt))
        args = events[2][1]; self.assertTrue(events[2][2]['check'])
        wanted = {'--mods': tc, '--pluto-bundle': pl, '--mixins-revision': 'a' * 40,
                  '--tconstruct-archive': tc.parent / 'tconstruct.hnmod',
                  '--pluto-archive': tc.parent / 'plutolib.hnmod',
                  '--tconstruct-archive-sha256': '1' * 64, '--pluto-archive-sha256': '2' * 64}
        for flag, value in wanted.items():
            self.assertEqual(args.count(flag), 1); self.assertEqual(args[args.index(flag) + 1], value)
        head.assert_called_once_with(['git', '-C', '../horizon-mixins', 'rev-parse', 'HEAD'], text=True)

    def test_workflow_package_or_verify_failure_never_stages(self):
        for phase in ('prepare', 'verify_prepared'):
            def fail(*args): raise ValueError('fixture admission rejected')
            module = types.SimpleNamespace(prepare=lambda *a: (None, None, None), verify_prepared=lambda *a: None)
            setattr(module, phase, fail)
            with self.subTest(phase=phase), patch.dict(sys.modules, prepare_playtest_bundles=module), \
                 patch.dict(os.environ, RUNNER_TEMP='/fixture'), patch.object(subprocess, 'run') as run, \
                 patch.object(subprocess, 'check_output') as head:
                with self.assertRaisesRegex(ValueError, 'admission rejected'):
                    exec(compile(workflow_python(), str(WORKFLOW), 'exec'), {})
                run.assert_not_called(); head.assert_not_called()

    def test_every_fixture_stage_call_uses_explicit_archive_builder_and_copy_closure(self):
        source = (ROOT / 'tconstruct/tests/test_stage.py').read_text(encoding='utf-8'); tree = ast.parse(source)
        calls = [n for n in ast.walk(tree) if isinstance(n, ast.Call) and ast.unparse(n.func) == 'subprocess.run']
        self.assertEqual(len(calls), 12)
        for call in calls:
            self.assertIsInstance(call.args[0], ast.List)
            first = call.args[0].elts[0]; self.assertIsInstance(first, ast.Starred)
            self.assertEqual(ast.unparse(first.value), 'stage_args()')
        self.assertIn("'--' + flag + '-archive'", source)
        self.assertIn("'--' + flag + '-archive-sha256'", source)
        self.assertIn('hashlib.sha256(raw).hexdigest()', source)
        for name in ('portable_archive.py', 'stage_package_contract.py', 'prepare_playtest_bundles.py', 'playtest_pluto_inputs.py'):
            self.assertIn(repr(name), source)
        self.assertIn("guards / 'check_recipe_placement_plan.py'", source)
        self.assertIn('plan, workspace, contract_path =', source)
        self.assertIn("contract['mode'] == 'full-tconstruct'", source)


if __name__ == '__main__': unittest.main()
