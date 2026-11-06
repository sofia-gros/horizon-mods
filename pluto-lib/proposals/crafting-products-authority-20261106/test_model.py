import copy
import json
from pathlib import Path
import unittest

from model import Operation, Owner, Pair, preview


class ProtocolOracle(unittest.TestCase):
    def owner(self, key='world-A/menu-1'):
        return Owner(key, ('tool', None, 'bottle'))

    def provider(self):
        calls = []
        def prepare():
            calls.append(len(calls) + 1)
            n = calls[-1]
            return Pair(f'tool-{n}', (None, None, f'glass-{n}'))
        return calls, prepare

    def test_stateful_provider_pair_is_used_once_without_recomputation(self):
        owner = self.owner(); calls, provider = self.provider()
        op = Operation(owner)
        self.assertTrue(op.prepare(provider))
        self.assertTrue(op.commit(owner))
        self.assertEqual(calls, [1])
        self.assertEqual(owner.committed, [Pair('tool-1', (None, None, 'glass-1'))])

    def test_replay_and_reprepare_cannot_grant_twice(self):
        owner = self.owner(); calls, provider = self.provider(); op = Operation(owner)
        op.prepare(provider); self.assertTrue(op.commit(owner))
        self.assertFalse(op.commit(owner)); self.assertFalse(op.prepare(provider))
        self.assertEqual(len(owner.committed), 1); self.assertEqual(calls, [1])

    def test_same_valued_other_owner_is_not_authority(self):
        owner = self.owner(); other = self.owner('world-B/menu-1')
        _, provider = self.provider(); op = Operation(owner); op.prepare(provider)
        self.assertFalse(op.commit(other)); self.assertEqual(other.committed, [])

    def test_aba_grid_mutation_invalidates(self):
        owner = self.owner(); _, provider = self.provider(); op = Operation(owner)
        op.prepare(provider); initial = owner.grid
        owner.edit_grid(('different', None, 'bottle')); owner.edit_grid(initial)
        self.assertFalse(op.commit(owner)); self.assertEqual(owner.committed, [])

    def test_destination_and_recipe_generations_invalidate(self):
        for field in ('destination_revision', 'recipe_revision'):
            with self.subTest(field=field):
                owner = self.owner(); _, provider = self.provider(); op = Operation(owner)
                op.prepare(provider); setattr(owner, field, 1)
                self.assertFalse(op.commit(owner)); self.assertEqual(owner.committed, [])

    def test_destroyed_owner_invalidates(self):
        owner = self.owner(); _, provider = self.provider(); op = Operation(owner)
        op.prepare(provider); owner.alive = False
        self.assertFalse(op.commit(owner)); self.assertEqual(owner.committed, [])

    def test_initially_invalid_owner_is_rejected_before_preparation(self):
        for changes in ({'alive': False}, {'crafts_available': 0}):
            with self.subTest(changes=changes):
                owner = self.owner()
                for key, value in changes.items(): setattr(owner, key, value)
                calls, provider = self.provider(); op = Operation(owner)
                self.assertFalse(op.prepare(provider)); self.assertEqual(calls, [])

    def test_allocation_cancel_and_unsupported_effects_abort_before_publication(self):
        for kwargs in ({'allocation_succeeds': False}, {'cancelled': True},
                       {'effects_supported': False}):
            with self.subTest(kwargs=kwargs):
                owner = self.owner(); before = copy.deepcopy(owner)
                _, provider = self.provider(); op = Operation(owner); op.prepare(provider)
                self.assertFalse(op.commit(owner, **kwargs)); self.assertEqual(owner, before)
                self.assertFalse(op.commit(owner))  # No revival of an aborted attempt.

    def test_deny_conflict_and_unsupported_route_do_not_invoke_provider(self):
        for kwargs in ({'admitted': False}, {'claimants': 2}, {'supported': False}):
            with self.subTest(kwargs=kwargs):
                calls, provider = self.provider(); op = Operation(self.owner())
                self.assertFalse(op.prepare(provider, **kwargs)); self.assertEqual(calls, [])
                self.assertEqual(op.state, 'aborted')

    def test_no_provider_only_selects_native_fallback(self):
        calls, provider = self.provider(); op = Operation(self.owner())
        self.assertFalse(op.prepare(provider, claimants=0)); self.assertEqual(calls, [])
        self.assertEqual(op.state, 'native')

    def test_preview_does_not_authorize_prediction(self):
        owner = self.owner(); calls, provider = self.provider()
        self.assertEqual(preview(provider), 'tool-1')
        op = Operation(owner, authoritative=False)
        self.assertFalse(op.prepare(provider)); self.assertFalse(op.commit(owner))
        self.assertEqual(calls, [1]); self.assertEqual(owner.committed, [])

    def test_action_time_pair_not_preview_output_is_published(self):
        owner = self.owner(); calls, provider = self.provider()
        self.assertEqual(preview(provider), 'tool-1')
        op = Operation(owner); op.prepare(provider); self.assertTrue(op.commit(owner))
        self.assertEqual(owner.committed[0], Pair('tool-2', (None, None, 'glass-2')))
        self.assertEqual(calls, [1, 2])

    def test_repeated_crafts_get_distinct_pairs(self):
        owner = self.owner(); calls, provider = self.provider()
        for _ in range(3):
            op = Operation(owner); self.assertTrue(op.prepare(provider))
            self.assertTrue(op.commit(owner))
        self.assertEqual(calls, [1, 2, 3]); self.assertEqual(owner.crafts_available, 0)
        self.assertEqual([p.output for p in owner.committed], ['tool-1', 'tool-2', 'tool-3'])
        self.assertEqual([p.remainders[-1] for p in owner.committed], ['glass-1', 'glass-2', 'glass-3'])

    def test_bad_pair_and_provider_error_have_no_publication(self):
        owner = self.owner()
        def failure(): raise ValueError('invalid proposal')
        for provider in (lambda: Pair('', (None, None, None)),
                         lambda: Pair('out', (None,)),
                         lambda: Pair('out', (None, 'fabricated', None)), failure):
            with self.subTest(provider=provider):
                op = Operation(owner); self.assertFalse(op.prepare(provider))
                self.assertFalse(op.commit(owner)); self.assertEqual(owner.committed, [])

    def test_reentrant_preparation_aborts_instead_of_producing_two_pairs(self):
        owner = self.owner(); op = Operation(owner); calls, provider = self.provider()
        def reentrant():
            self.assertFalse(op.prepare(provider))
            return Pair('outer', (None, None, 'glass'))
        self.assertFalse(op.prepare(reentrant)); self.assertFalse(op.commit(owner))
        self.assertEqual(calls, []); self.assertEqual(owner.committed, [])

    def test_acceptance_matrix_has_no_native_pass_claim(self):
        rows = json.loads(Path(__file__).with_name('acceptance.json').read_text())['required_native_tests']
        self.assertEqual(len(rows), len({row['id'] for row in rows}))
        self.assertTrue(all(row['status'] == 'not_run' for row in rows))
        self.assertEqual({r['family'] for r in rows},
                         {'protocol', 'local', 'menu', 'lan', 'crafter', 'compatibility'})


if __name__ == '__main__':
    unittest.main()
