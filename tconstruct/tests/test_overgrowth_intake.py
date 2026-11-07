"""Source/inverse checks only; no Rust, native Game, or gameplay execution."""
import hashlib
import json
from pathlib import Path
import re
import struct
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
MOD = 'tconstruct/mixins/tconstruct/'
PROOF = ROOT / 'tconstruct/tests/overgrowth-intake-proof.json'

def read(path):
    return (ROOT / path).read_text(encoding='utf-8')

def digest(data):
    return hashlib.sha256(data).hexdigest()

def restore_manifest(text, proof):
    tail = proof['manifest']['append']
    if text.count(tail) != 1 or not text.endswith(tail):
        raise ValueError('expected the exact single appended group')
    return text[:-len(tail)].encode()

class Source(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads(PROOF.read_text(encoding='utf-8'))
        cls.world = read(MOD + 'source-overgrowth-world.rs')
        cls.rule = read(MOD + 'source-overgrowth.rs')

    def test_manifest_whole_inverse_preserves_every_old_module_hook_state(self):
        p = self.proof['manifest']
        text = read(p['path'])
        self.assertEqual(digest(text.encode()), p['after_sha256'])
        old = restore_manifest(text, self.proof)
        self.assertEqual(digest(old), p['before_sha256'])
        self.assertEqual(hashlib.sha1(b'blob '+str(len(old)).encode()+b'\0'+old).hexdigest(), p['before_git_blob'])
        after, before = tomllib.loads(text), tomllib.loads(old.decode())
        self.assertEqual(after['module'][:-3], before['module'])
        self.assertEqual(after['injection'][:-1], before['injection'])
        self.assertEqual(after.get('state'), before.get('state'))

    def test_missing_duplicated_or_changed_group_rejects(self):
        p = self.proof['manifest']; text = read(p['path']); tail = p['append']
        for bad in [text+tail, text.replace(tail, ''), text.replace('source-modifier-random.rs','changed.rs')]:
            with self.assertRaises(ValueError): restore_manifest(bad, self.proof)

    def test_every_new_payload_is_fixed(self):
        self.assertEqual(len(self.proof['new_payloads']), 4)
        for path, record in self.proof['new_payloads'].items():
            self.assertEqual(digest(read(path).encode()), record['sha256'])

    def test_one_real_slot_hook_no_phase_state_or_second_scan(self):
        after = tomllib.loads(read(self.proof['manifest']['path']))
        hook = after['injection'][-1]
        self.assertEqual(hook['target'], "horizon_client::game::native_hooks::Game<'_>::native_inventory_tick")
        self.assertEqual(hook['kind'], 'TAIL')
        self.assertEqual(hook['bindings'][1], {'name':'slot', 'ty':'i32', 'value':'slot'})
        self.assertIn('tick(game, slot)', read(MOD+'source-overgrowth-tick.rs'))
        self.assertNotIn('for slot', self.world)
        self.assertNotIn('overgrowth_phase', self.world)
        self.assertNotIn('native_acting_id', self.world)
        self.assertIn('player.health.tick_count', self.world)
        self.assertIn('tick_count % 20 == 0', self.rule)
        self.assertIn('native_on("tconstruct")', self.world)

    def test_current_item_tags_definition_and_traits_remain_policy_owned(self):
        for expected in ['__horizon_mixins_tconstruct_resolved_item_tags', 'tconstruct:modifiable',
                         '__horizon_mixins_tconstruct_modifier_resource_snapshot',
                         'tconstruct:tinkering/modifiers/overgrowth', 'tconstruct_source_material_traits::levels',
                         'level_with_upgrade', 'tconstruct:overworked']:
            self.assertIn(expected, self.world)
        self.assertIn('stack.count != 1', self.world)
        self.assertIn('capacity <= 32767', self.rule)
        self.assertIn('0.05_f32 * level as f32', self.rule)
        self.assertIn('used == 40', self.rule)
        self.assertIn('slot < 36', self.rule)

    def test_existing_pluto_patch_is_the_only_compound_writer(self):
        body = self.world.split('#[cfg(test)]', 1)[0]
        self.assertEqual(body.count('pluto_nbt_storage::patch_compound('), 1)
        self.assertIn('[("tic_overslime", Some(Tag::Int(next)))]', body)
        self.assertNotIn('fields.insert', body)
        self.assertEqual(body.count('set_other('), 1)
        self.assertLess(body.index('let Some(next) = prepare'), body.index('set_other('))
        self.assertNotIn('insert(', body)

    def test_rng_draw_occurs_only_after_eligibility_and_is_not_world_random(self):
        body = self.world.split('#[cfg(test)]', 1)[0]
        self.assertLess(body.index('if !rule::eligible'), body.index('overworked, draw()'))
        self.assertEqual(body.count('draw()'), 1)
        self.assertNotIn('world.random', body)
        random = read(MOD+'source-modifier-random.rs')
        self.assertEqual(random.count('static RANDOM:'), 1)
        self.assertIn('Mutex<LegacyRandom>', random)
        self.assertIn('.next_float()', random)
        self.assertNotIn('Xoroshiro', random)

    def test_prepared_native_rust_cases_have_exact_inventory(self):
        inventory = {}
        for file, count in [('source-overgrowth.rs', 6), ('source-overgrowth-world.rs', 4), ('source-modifier-random.rs', 1)]:
            names = re.findall(r'#\[test\]\s*fn ([a-z_]+)\(', read(MOD+file))
            self.assertEqual(len(names), count)
            self.assertEqual(len(set(names)), count)
            inventory[file] = names
        self.assertEqual(sum(map(len, inventory.values())), 11)

    def test_java_float_known_vectors_are_not_rounded_decimal_or_xoroshiro(self):
        # Independent integer Java LCG arithmetic checks the constants in the
        # prepared actual-native Rust test. It does not execute LegacyRandom.
        source = read(MOD+'source-modifier-random.rs')
        match = re.search(r'for numerator in \[(.*?)\]', source)
        expected = [int(n.strip().replace('_u32','').replace('_','')) for n in match.group(1).split(',')]
        state = 0x5deece66d
        observed = []
        for _ in range(4):
            state = (state * 0x5deece66d + 11) & ((1 << 48) - 1)
            observed.append(state >> 24)
        self.assertEqual(observed, expected)
        self.assertTrue(all(0 <= n < (1 << 24) for n in observed))

    def test_source_docs_leave_type_gameplay_and_stream_parity_unproved(self):
        docs = read('tconstruct/docs/overgrowth-pluto-intake.md')
        for text in ['Rust/native/plan/gameplay remain unrun', 'No PR closure', 'startup seed', '32767', 'same-mode', 'native inventory callback']:
            self.assertIn(text, docs)
        self.assertIn('None => 0', self.world)
        self.assertIn('_ => return None', self.world)

if __name__ == '__main__': unittest.main()
