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
        self.assertTrue(documentation_only(['README.md', 'docs/roadmap.md', 'docs/preview.png']))

    def test_runtime_tests_configuration_and_empty_are_not_docs_only(self):
        for paths in ([], ['mixins/tconstruct/transaction.rs'], ['tests/test_ci_scope.py'],
                      ['compatibility.json'], ['.github/workflows/mixins.yml'],
                      ['docs/upstream-tags.sha256'], ['docs/example.py'],
                      ['README.md', 'tools/stage.py']):
            self.assertFalse(documentation_only(paths), paths)

    def test_prior_success_is_required_and_only_docs_skip(self):
        event = {'pull_request': {'number': 13, 'head': {'sha': 'head', 'ref': 'branch', 'repo': {'full_name': 'a/b'}}, 'base': {'sha': 'base'}}}
        runs = {'workflow_runs': [{'id': 3, 'head_sha': 'old', 'pull_requests': [{'number': 13}]}]}
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'event.json'
            path.write_text(json.dumps(event))
            env = {'GITHUB_EVENT_PATH': str(path), 'GITHUB_REPOSITORY': 'a/b'}
            for changed, expected in ((b'docs/readme.md\0', True), (b'tools/stage.py\0', False)):
                responses = [subprocess.CompletedProcess([], 0, json.dumps(runs)), subprocess.CompletedProcess([], 0, changed)]
                with patch.dict(os.environ, env), patch.object(ci_scope, 'ancestor', return_value=True), patch.object(subprocess, 'run', side_effect=responses):
                    self.assertEqual(ci_scope.main(), expected)
            with patch.dict(os.environ, env), patch.object(ci_scope, 'ancestor', return_value=False), patch.object(subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, json.dumps(runs))):
                self.assertFalse(ci_scope.main())
