"""Consumer/Pluto guard regressions, not native planning or live UI proof."""
import copy
import importlib.util
from pathlib import Path
import tomllib
import unittest

REPO = Path(__file__).resolve().parents[2]
ROOT = REPO / 'tconstruct/mixins/tconstruct'
PLUTO = REPO / 'pluto-lib'


def load(name):
    spec = importlib.util.spec_from_file_location(name, PLUTO / 'tools' / f'{name}.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


book = load('check_current_book_plan')
payload = load('check_client_payload_plan')


class GuidePlutoComposition(unittest.TestCase):
    def manifest(self):
        return tomllib.loads((ROOT / 'horizon-mixin.toml').read_text())

    def book_plan(self):
        meta = book.metadata()
        return {'inputs': meta['official_sources'],
                'injections': copy.deepcopy(self.manifest()['injection']),
                'modules': [{'mod_id': 'pluto_native_creative', 'module_id': meta['module_id'],
                    'file': meta['module_file'], 'name': meta['module_name'],
                    'source': (PLUTO / 'mixins/pluto_native_creative' / meta['source']).read_text()}]}

    def payload_plan(self):
        meta = payload.metadata()
        return {'inputs': meta['source_sha256'], 'injections': [{
            'mod_id': 'pluto_native_creative', 'injection_id': meta['injection_id'],
            'target': meta['target'], **meta['location'], 'kind': 'RETURN', 'cancellable': False,
            'function_fingerprint': meta['function_fingerprint'], 'bindings': meta['bindings'],
            'helper_source': (PLUTO / 'mixins/pluto_native_creative' / meta['helper']).read_text(),
        }, *copy.deepcopy(self.manifest()['injection'])]}

    def test_manifest_composition_no_longer_replaces_native_turn(self):
        # These are real current manifest entries, not an assertion that a
        # complete official AST plan/apply or every other guard passed.
        book.check(self.book_plan(), book.metadata())
        manifest = self.manifest()
        module = next(m for m in manifest['module'] if m['id'] == 'materials-book-navigation')
        self.assertEqual(module['file'], 'crates/horizon-client/src/game/books.rs')
        self.assertEqual(module['source'], 'source-materials-book-navigation.rs')
        self.assertFalse((ROOT / 'source-materials-book-turn.rs').exists())
        for name in ('key', 'click', 'control', 'navigation'):
            source = (ROOT / f'source-materials-book-{name}.rs').read_text()
            self.assertNotIn('game.turn_page(', source)
            self.assertNotIn('game.book_control(', source)
        control = next(h for h in manifest['injection'] if h['id'] == 'materials-book-control')
        self.assertEqual(control['target'], "horizon_client::game::books::Game<'_>::book_control")
        self.assertEqual(control['bindings'][1], {'name': 'action', 'ty': 'Option<&str>', 'value': 'action'})

    def test_restoring_global_turn_or_alias_still_fails_closed(self):
        meta = book.metadata()
        for target in (meta['turn_target'], 'alias::turn'):
            plan = self.book_plan()
            plan['injections'].append({'target': target, 'kind': 'HEAD', 'cancellable': True,
                                       'function_fingerprint': meta['turn_fingerprint']})
            with self.assertRaisesRegex(ValueError, 'page-turn hooks'):
                book.check(plan, meta)

    def test_remote_feature_is_preserved_and_still_blocks_full_composition(self):
        manifest = self.manifest()
        response = next(h for h in manifest['injection'] if h['id'] == 'materials-book-response')
        request = next(h for h in manifest['injection'] if h['id'] == 'materials-book-request')
        self.assertEqual((response['kind'], response['cancellable']), ('HEAD', True))
        self.assertEqual(request['helper'], 'source-materials-book-request.rs')
        self.assertIn('game.send(S::CustomPayload(', (ROOT / 'source-materials-book.rs').read_text())
        with self.assertRaisesRegex(ValueError, 'bypass or reorder'):
            payload.check(self.payload_plan(), payload.metadata())

    def test_navigation_only_composition_preserves_payload_guard(self):
        plan = self.payload_plan()
        plan['injections'] = [plan['injections'][0], *[
            h for h in self.manifest()['injection']
            if h['id'] in {'materials-book-key', 'materials-book-click', 'materials-book-control'}]]
        payload.check(plan, payload.metadata())


if __name__ == '__main__':
    unittest.main()
