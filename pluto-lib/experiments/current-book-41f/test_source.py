"""Pure source/inverse/guard checks; no actual plan, rustfmt, Cargo or native run."""
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
BASE = Path(os.environ['GUIDE_BASE_ROOT']) / 'pluto-lib'
OLD = Path(os.environ['GUIDE_624C_ROOT'])
LATEST = Path(os.environ['GUIDE_41F_ROOT'])
GAME = Path(os.environ['GUIDE_41F_GAME'])
E = json.loads((HERE / 'evidence.json').read_bytes())
META = HERE.parents[1] / 'tools/current-book-source.json'
sha = lambda b: hashlib.sha256(b).hexdigest()
spec = importlib.util.spec_from_file_location('unchanged_current_book_guard', BASE / 'tools/check_current_book_plan.py')
guard = importlib.util.module_from_spec(spec); spec.loader.exec_module(guard)


def native(path):
    return GAME if path == 'crates/horizon-client/src/game.rs' else LATEST / path


def model_plan(meta):
    return {'modules': [{'mod_id': 'pluto_native_creative', 'module_id': 'pluto_native_creative-current-book',
        'file': meta['module_file'], 'name': meta['module_name'],
        'source': (BASE / 'mixins/pluto_native_creative/current-book.rs').read_text(encoding='utf-8')}],
        'injections': [], 'inputs': dict(meta['official_sources'])}


class Checks(unittest.TestCase):
    def test_metadata_exact_revision_and_source_only_inverse(self):
        old = (BASE / 'tools/current-book-source.json').read_bytes(); new = META.read_bytes()
        self.assertEqual(sha(old), E['before_sha256']); self.assertEqual(sha(new), E['after_sha256'])
        a = json.loads(old); b = json.loads(new)
        for key in ('horizon_revision', 'mixins_revision', 'official_sources'): b[key] = a[key]
        self.assertEqual((json.dumps(b, indent=2) + '\n').encode(), old)

    def test_four_exact_official_sources(self):
        for path, expected in E['official_sources'].items():
            raw = native(path).read_bytes()
            self.assertEqual(len(raw), expected['bytes']); self.assertEqual(sha(raw), expected['sha256'])
            self.assertEqual(hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest(), expected['git_blob'])

    def test_complete_book_inverse_changes_only_native_background_drawing(self):
        path = 'crates/horizon-client/src/game/books.rs'
        old = (OLD / path).read_bytes(); new = native(path).read_text(encoding='utf-8')
        self.assertEqual(sha(old), E['old_book_sha256'])
        start = E['book_inverse_scope']; self.assertEqual(new.count(start), 1)
        at = new.index(start); tail = new[at:]
        for before, after in reversed(E['book_draw_edits']):
            self.assertEqual(tail.count(after), 1); tail = tail.replace(after, before, 1)
        self.assertEqual((new[:at] + tail).encode(), old)

    def test_turn_page_native_body_is_unchanged(self):
        path = 'crates/horizon-client/src/game/books.rs'; result = []
        for root in (OLD, LATEST):
            text = (root / path).read_text(encoding='utf-8'); prefix = '    fn turn_page('
            self.assertEqual(text.count(prefix), 1); start = text.index(prefix)
            body = text[start:text.index('\n    }', start) + len('\n    }')].encode()
            self.assertEqual(sha(body), E['unchanged_turn_page_sha256']); result.append(body)
        self.assertEqual(*result)

    def test_runtime_and_guard_are_not_changed(self):
        self.assertEqual(sha((BASE / 'mixins/pluto_native_creative/current-book.rs').read_bytes()), E['helper_sha256'])
        self.assertEqual(sha((BASE / 'tools/check_current_book_plan.py').read_bytes()), E['guard_sha256'])
        self.assertFalse(E['runtime_changed']); self.assertFalse(E['guard_algorithm_changed'])

    def test_same_guard_accepts_only_exact_synthetic_module_and_source(self):
        meta = json.loads(META.read_bytes()); plan = model_plan(meta); guard.check(plan, meta)
        for kind in ('missing', 'duplicate', 'source', 'placement', 'official'):
            p = copy.deepcopy(plan)
            if kind == 'missing': p['modules'] = []
            elif kind == 'duplicate': p['modules'].append(copy.deepcopy(p['modules'][0]))
            elif kind == 'source': p['modules'][0]['source'] += '\n'
            elif kind == 'placement': p['modules'][0]['file'] = 'other.rs'
            else: p['inputs'][meta['module_file']] = 'wrong'
            with self.assertRaises(ValueError): guard.check(p, meta)

    def test_native_turn_hooks_remain_forbidden_including_alias_fingerprint(self):
        meta = json.loads(META.read_bytes())
        for row in ({'target': meta['turn_target'], 'function_fingerprint': 'other'},
                    {'target': 'alias', 'function_fingerprint': meta['turn_fingerprint']}):
            p = model_plan(meta); p['injections'].append(row)
            with self.assertRaises(ValueError): guard.check(p, meta)

    def test_prompt_and_new_screen_admission_are_not_widened(self):
        helper = (BASE / 'mixins/pluto_native_creative/current-book.rs').read_text(encoding='utf-8')
        game = GAME.read_text(encoding='utf-8')
        for term in ('!game.dialog_open()', 'game.command_prompt.is_none()', 'game.link_prompt.is_none()',
                     'game.code_of_conduct.is_none()', 'game.server_packs.prompt.is_none()',
                     'game.screen == crate::game::Screen::Playing'):
            self.assertIn(term, helper)
        for variant in ('EmoteWheel,', 'HudEditor,'): self.assertIn(variant, game)
        self.assertFalse(E['actual_full_guard_rerun']); self.assertFalse(E['native_execution'])
        self.assertFalse(E['payload_node_observed']); self.assertEqual(E['actual_run'], 42231588813)


if __name__ == '__main__': unittest.main()
