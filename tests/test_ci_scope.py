import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from tools import ci_scope

documentation_only = ci_scope.documentation_only


class Scope(unittest.TestCase):
    def test_documentation_and_images(self):
        self.assertTrue(documentation_only(['README.md', 'docs/roadmap.md', 'docs/preview.png', 'tconstruct/README.md', 'tconstruct/docs/playing.md', 'tconstruct/docs/preview.png']))

    def test_runtime_tests_configuration_and_empty_are_not_docs_only(self):
        for paths in ([], ['mixins/tconstruct/transaction.rs'], ['tests/test_ci_scope.py'],
                      ['compatibility.json'], ['.github/workflows/mixins.yml'],
                      ['docs/upstream-tags.sha256'], ['docs/example.py'],
                      ['README.md', 'tools/stage.py'], ['tconstruct/docs/example.py'], ['tconstruct/mixins/example.rs']):
            self.assertFalse(documentation_only(paths), paths)

    def test_renaming_code_into_docs_still_requires_checks(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            def git(*args):
                return subprocess.check_output(['git', '-C', directory, *args])
            git('init', '-q')
            git('config', 'user.name', 'Fixture')
            git('config', 'user.email', 'fixture@example.invalid')
            (repo / 'code.py').write_text('unchanged content\n')
            git('add', 'code.py')
            git('commit', '-qm', 'code')
            (repo / 'docs').mkdir()
            (repo / 'code.py').rename(repo / 'docs/code.md')
            git('add', '-A')
            git('commit', '-qm', 'rename')
            changed = git('diff', '--no-renames', '--name-only', '-z', 'HEAD^', 'HEAD')
            self.assertFalse(documentation_only([path.decode() for path in changed.split(b'\0') if path]))

    def test_prior_success_is_required_and_only_docs_skip(self):
        event = {'pull_request': {'number': 13, 'head': {'sha': 'head', 'ref': 'branch', 'repo': {'full_name': 'a/b'}}, 'base': {'sha': 'base'}}}
        runs = {'workflow_runs': [{'id': 3, 'head_sha': 'old', 'pull_requests': [{'number': 13}]}]}
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'event.json'
            path.write_text(json.dumps(event))
            env = {'GITHUB_EVENT_PATH': str(path), 'GITHUB_REPOSITORY': 'a/b'}
            for changed, expected in ((b'docs/readme.md\0', True), (b'tools/stage.py\0', False)):
                responses = [subprocess.CompletedProcess([], 0, json.dumps(runs)), subprocess.CompletedProcess([], 0, changed)]
                if expected:
                    responses.append(subprocess.CompletedProcess([], 0, 'test\tDetermine validation scope\ttime CI_VERIFIED_BASE=base\n'))
                with patch.dict(os.environ, env), patch.object(ci_scope, 'ancestor', return_value=True) as ancestry, patch.object(subprocess, 'run', side_effect=responses) as run:
                    self.assertEqual(ci_scope.main(), expected)
                    ancestry.assert_called_once_with('old', 'head')
                    self.assertIn('--no-renames', run.call_args_list[1].args[0])
            responses = [subprocess.CompletedProcess([], 0, json.dumps(runs)), subprocess.CompletedProcess([], 0, b'docs/readme.md\0'), subprocess.CompletedProcess([], 0, 'test\tDetermine validation scope\ttime CI_VERIFIED_BASE=other\n')]
            with patch.dict(os.environ, env), patch.object(ci_scope, 'ancestor', return_value=True), patch.object(subprocess, 'run', side_effect=responses):
                self.assertFalse(ci_scope.main())
            with patch.dict(os.environ, env), patch.object(ci_scope, 'ancestor', return_value=False), patch.object(subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, json.dumps(runs))):
                self.assertFalse(ci_scope.main())
            unrelated = {'workflow_runs': [{'id': 4, 'head_sha': 'old', 'pull_requests': [{'number': 14}]}]}
            with patch.dict(os.environ, env), patch.object(subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, json.dumps(unrelated))):
                self.assertFalse(ci_scope.main())
            with patch.dict(os.environ, env), patch.object(subprocess, 'run', return_value=subprocess.CompletedProcess([], 1, '')):
                self.assertFalse(ci_scope.main())
            event['pull_request']['head']['repo']['full_name'] = 'other/fork'
            path.write_text(json.dumps(event))
            with patch.dict(os.environ, env), patch.object(subprocess, 'run') as run:
                self.assertFalse(ci_scope.main())
                run.assert_not_called()
            event['pull_request']['head']['repo'] = None
            path.write_text(json.dumps(event))
            with patch.dict(os.environ, env):
                self.assertFalse(ci_scope.main())
