"""Small synthetic contracts test rejection, not real native integration."""
import copy
import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('block_entity_plan', ROOT / 'tools/check_block_entity_plan.py')
GUARD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GUARD)


def pending_metadata():
    # Exercise pending-policy rejection independently of production review state.
    return dict(GUARD.metadata(), review_status='pending_native_attachment_review')


class BlockEntityPlan(unittest.TestCase):
    def test_spawn_state_aliases_cannot_omit_native_attachment_review(self):
        for entry in [
            {'injections':[{'injection_id':'alias','target':"sunrise_client::game::native_hooks::Game<'_>::native_entity_join_level"}]},
            {'injections':[{'injection_id':'alias','target':"sunrise_client::game::commands::data::Game<'_>::apply_entity_nbt"}]},
            {'injections':[{'injection_id':'alias','target':'sunrise_client::mobs::finalize::set_base'}]},
            {'injections':[{'injection_id':'alias','target':'sunrise_client_lib::mobs::finalize::set_base'}]},
            {'injections':[{'injection_id':'alias','target':'sunrise_client::mobs::finalize::compute'}]},
            {'injections':[{'injection_id':'alias','target':'sunrise_client_lib::mobs::finalize::compute'}]},
            {'injections':[{'injection_id':'alias','target':'sunrise_client::mobs::finalize::remove_modifier'}]},
            {'modules':[{'module_id':'alias','file':'crates/horizon-client/src/mobs/finalize.rs','name':'pluto_living_attribute_ops'}]},
            {'modules':[{'module_id':'alias','file':'crates/horizon-client/src/entry.rs','name':'pluto_living_load_access'}]},
            {'injections':[{'injection_id':'alias','target':'sunrise_client::mobs::living::Mob::cause_fall_damage_at'}]},
            {'injections':[{'injection_id':'alias','target':'sunrise_client_lib::mobs::living::Mob::cause_fall_damage_at'}]},
            {'injections':[{'injection_id':'alias','target':'sunrise_client::mobs::living::Mob::jump_power'}]},
            {'injections':[{'injection_id':'alias','target':'sunrise_client_lib::mobs::living::Mob::jump_power'}]},
            {'injections':[{'injection_id':'alias','target':'sunrise_client::mobs::cube::attack_damage'}]},
            {'injections':[{'injection_id':'alias','target':'sunrise_client_lib::mobs::cube::attack_damage'}]},
            {'modules':[{'module_id':'alias','file':'crates/horizon-assets/src/item_model.rs','name':'alias'}]},
            {'modules':[{'module_id':'alias','file':'crates/horizon-client/src/item_render/special.rs','name':'alias'}]},
            {'injections':[{'injection_id':'alias','target':'sunrise_assets::item_model::SpecialModel::decode'}]},
            {'injections':[{'injection_id':'alias','target':'sunrise_client::item_render::special::submit'}]},
            {'injections':[{'injection_id':'alias','target':'sunrise_client::game::armor_stands::skull_kind'}]},
            {'modules':[{'module_id':'alias','file':'crates/horizon-client/src/mod_content.rs','name':'alias'}]},
            {'injections':[{'injection_id':'alias','target':'sunrise_client::mod_content::Plan::override_gameplay'}]},
            {'injections':[{'injection_id':'alias','target':"sunrise_client_lib::mod_content::Plan<'_>::override_gameplay"}]},
            {'states':[{'state_id':'alias','target':'sunrise_client_lib::mobs::nav::Navigation'}]},
            {'states':[{'state_id':'alias','target':'sunrise_client::game::combat::CombatTracker'}]},
            {'modules':[{'module_id':'alias','file':'crates/horizon-client/src/game/combat.rs','name':'alias'}]},
            {'states':[{'state_id':'alias','constructors':[{'file':'crates/horizon-client/src/game/combat.rs'}]}]},
            {'modules':[{'module_id':'alias','file':'crates/horizon-client/src/mobs/nav.rs','name':'alias'}]},
            {'states':[{'state_id':'alias','constructors':[{'file':'crates/horizon-client/src/mobs/nav.rs'}]}]},
            {'states':[{'state_id':'alias','target':'sunrise_client::mobs::Mob'}]},
            {'modules':[{'module_id':'alias','file':'crates/horizon-client/src/mobs.rs','name':'alias'}]},
            {'states':[{'state_id':'alias','constructors':[{'file':'crates/horizon-client/src/mobs.rs'}]}]},
        ]:
            self.assertTrue(GUARD.kept(entry))
            with self.assertRaisesRegex(ValueError,'native attachment'):
                GUARD.check(entry,self.workspace,meta=pending_metadata())

    def test_spawn_caller_aliases_and_missing_full_contract_stay_blocked(self):
        invalid=copy.deepcopy(self.plan)
        invalid['states'].append({'state_id':'alias','target':'sunrise_client::mobs::Mob'})
        with self.assertRaisesRegex(ValueError,'Externally constructible Mob'):
            self.check(invalid)
        for target in GUARD.SPAWN_TARGETS:
            plan={'injections':[{'injection_id':'alias','target':target}]}
            self.assertTrue(GUARD.kept(plan))
            with self.assertRaisesRegex(ValueError,'native attachment'):
                GUARD.check(plan,self.workspace,meta=pending_metadata())
        meta=copy.deepcopy(self.meta)
        meta['modules']=[m for m in meta['modules'] if m['name']!='pluto_entity_spawn_game']
        with self.assertRaisesRegex(ValueError,'mob spawn boundary'):
            self.check(meta=meta)
        plan=copy.deepcopy(self.plan)
        entry=next(i for i in plan['injections'] if i['injection_id']=='pluto-entity-spawn-add')
        entry['kind']='HEAD'
        with self.assertRaises(ValueError):
            self.check(plan)

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.workspace = Path(self.temp.name) / 'official'
        self.helpers = Path(self.temp.name) / 'helpers'
        self.helpers.mkdir()
        self.meta = pending_metadata()
        self.meta['review_status'] = 'reviewed'
        # This fixture materializes synthetic AST sites, not direct source edits.
        # The separate direct-proof test below supplies its own exact receipt.
        self.meta.pop('direct_attachments', None)
        self.meta.pop('direct_helpers', None)
        self.meta.pop('direct_review_status', None)
        for injection in self.meta['injections']:
            injection['function_fingerprint'] = '1' * 64
        for state in self.meta['states']:
            state.update(fingerprint='2' * 64, node=[0])
            for constructor in state['constructors']:
                constructor.update(function_fingerprint='3' * 64, fingerprint='4' * 64,
                                   node=[1, 0], expression=0)
        for relative in self.meta['official_source_sha256']:
            path = self.workspace / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            content = ('synthetic authority ' + relative).encode()
            path.write_bytes(content)
            self.meta['official_source_sha256'][relative] = GUARD.CONTRACT.digest(content)
        self.plan = {'schema': 6, 'inputs': dict(self.meta['official_source_sha256']),
                     'modules': [], 'injections': [],
                     'states': [dict(copy.deepcopy(state), mod_id='pluto_lib') for state in self.meta['states']]}
        for section, content_key in [('modules', 'source'), ('injections', 'helper_source')]:
            for required in self.meta[section]:
                content = 'synthetic helper ' + required['source_file']
                (self.helpers / required['source_file']).write_bytes(content.encode())
                required['source_sha256'] = GUARD.CONTRACT.digest(content.encode())
                entry = {k: copy.deepcopy(v) for k, v in required.items()
                         if k not in ('source_file', 'source_sha256')}
                entry.update(mod_id='pluto_lib')
                entry[content_key] = content
                self.plan[section].append(entry)

    def check(self, plan=None, meta=None):
        GUARD.check(self.plan if plan is None else plan, self.workspace,
                    self.meta if meta is None else meta, self.helpers)

    def test_root_gate_and_local_adapter_are_exact_and_third_hooks_never_pass(self):
        self.check()
        plan = copy.deepcopy(self.plan)
        local = next(e for e in plan['injections'] if e['injection_id'] == 'pluto-item-placement-root-path')
        local['variable']['at'] = 'LOAD'
        with self.assertRaises(ValueError):
            self.check(plan)
        plan = copy.deepcopy(self.plan)
        gate = next(e for e in plan['injections'] if e['injection_id'] == 'pluto-item-placement-root-gate')
        plan['injections'].append(dict(gate, injection_id='unreviewed-root-hook'))
        with self.assertRaises(ValueError):
            self.check(plan)

    def test_mask_boundary_cannot_escape_pending_or_missing_attachment_review(self):
        for entry, section in [
            ({'injection_id': 'alias', 'target': GUARD.MASK_TARGET}, 'injections'),
            ({'module_id': 'alias', 'file': 'crates/horizon-world/src/level/mod.rs',
              'name': 'pluto_item_behavior_access'}, 'modules'),
        ]:
            plan = copy.deepcopy(self.plan)
            plan[section].append(entry)
            self.assertTrue(GUARD.kept(plan))
            meta = copy.deepcopy(self.meta)
            meta['injections'] = [i for i in meta['injections'] if i.get('target') != GUARD.MASK_TARGET]
            meta['modules'] = [m for m in meta['modules'] if m.get('name') != 'pluto_item_behavior_access']
            with self.assertRaisesRegex(ValueError, 'ender-mask boundary'):
                self.check(plan, meta)
            with self.assertRaisesRegex(ValueError, 'native attachment'):
                self.check(plan, pending_metadata())

    def test_trade_boundary_requires_review_and_rejects_foreign_private_edits(self):
        self.check()
        for entry, section in [
            ({'injection_id': 'alias', 'target': GUARD.TRADE_TARGET}, 'injections'),
            ({'module_id': 'alias', 'file': 'crates/horizon-client/src/merchant.rs', 'name': 'alias'}, 'modules'),
        ]:
            plan = copy.deepcopy(self.plan)
            plan[section].append(entry)
            self.assertTrue(GUARD.kept(plan))
            with self.assertRaises(ValueError):
                self.check(plan)
            with self.assertRaisesRegex(ValueError, 'native attachment'):
                self.check(plan, pending_metadata())
        meta = copy.deepcopy(self.meta)
        meta['injections'] = [i for i in meta['injections'] if i.get('target') != GUARD.TRADE_TARGET]
        with self.assertRaisesRegex(ValueError, 'Trade pool boundary'):
            self.check(meta=meta)

    def test_reload_registry_aliases_require_native_attachment_review(self):
        for entry, section in [
            ({'injection_id': 'alias', 'target': GUARD.RELOAD_TARGET}, 'injections'),
            ({'module_id': 'alias', 'file': 'crates/horizon-client-core/src/registries.rs', 'name': 'alias'}, 'modules'),
            ({'module_id': 'alias', 'file': 'crates/horizon-client/src/game/chat.rs', 'name': 'alias'}, 'modules'),
            ({'injection_id': 'alias', 'target': 'sunrise_items::items::Items::load_with'}, 'injections'),
            ({'module_id': 'alias', 'file': 'crates/horizon-items/src/items.rs', 'name': 'alias'}, 'modules'),
            ({'injection_id': 'alias', 'target': 'sunrise_client::item_render::ItemRenderer::resolve_with_rule'}, 'injections'),
            ({'injection_id': 'alias', 'target': 'sunrise_client_lib::item_render::ItemRenderer::resolve_with_rule'}, 'injections'),
            ({'injection_id': 'alias', 'target': "sunrise_client::game::Game<'_>::particle_sprite"}, 'injections'),
            ({'injection_id': 'alias', 'target': "sunrise_client::game::native_client_particles::Game<'_>::native_add_particle"}, 'injections'),
            ({'module_id': 'alias', 'file': 'crates/horizon-client/src/game.rs', 'name': 'pluto_item_particles'}, 'modules'),
            ({'injection_id': 'alias', 'target': 'sunrise_client_lib::mobs::cube::landing_splash'}, 'injections'),
            ({'injection_id': 'alias', 'target': 'sunrise_client::mobs::cube::landing_splash'}, 'injections'),
            ({'injection_id': 'alias', 'target': "sunrise_client::game::creatures::Game<'_>::handle_mob_events"}, 'injections'),
            ({'module_id': 'alias', 'file': 'crates/horizon-client/src/mobs/cube.rs', 'name': 'alias'}, 'modules'),
            ({'injection_id': 'alias', 'target': "sunrise_client::game::creatures::Game<'_>::drop_death_loot"}, 'injections'),
            ({'injection_id': 'alias', 'target': "sunrise_client::game::creatures::Game<'_>::tick_creature_entities"}, 'injections'),
            ({'injection_id': 'alias', 'target': 'sunrise_client_lib::mobs::drops::roll'}, 'injections'),
            ({'injection_id': 'alias', 'target': 'sunrise_client::mobs::drops::roll'}, 'injections'),
            ({'injection_id': 'alias', 'target': 'sunrise_client::game::native_hooks::mob_kind'}, 'injections'),
            ({'module_id': 'alias', 'file': 'crates/horizon-client/src/game.rs', 'name': 'pluto_living_drops'}, 'modules'),
            ({'module_id': 'alias', 'file': 'crates/horizon-client/src/mobs/drops.rs', 'name': 'alias'}, 'modules'),
            ({'module_id': 'alias', 'file': 'crates/horizon-client-render/src/mesher.rs', 'name': 'pluto_block_colors'}, 'modules'),
            ({'module_id': 'alias', 'file': 'crates/horizon-client/src/item_render.rs', 'name': 'alias'}, 'modules'),
        ]:
            plan = {section: [entry]}
            self.assertTrue(GUARD.kept(plan))
            with self.assertRaisesRegex(ValueError, 'native attachment'):
                self.check(plan, pending_metadata())
        legacy = copy.deepcopy(self.plan)
        legacy['injections'].append({'injection_id': 'old-reload-alias', 'target': GUARD.RELOAD_APPLY_TARGET})
        self.assertTrue(GUARD.kept(legacy))
        with self.assertRaisesRegex(ValueError, 'apply-only'):
            self.check(legacy)
        meta = copy.deepcopy(self.meta)
        meta['injections'] = [i for i in meta['injections'] if i.get('target') != GUARD.RELOAD_TARGET]
        with self.assertRaisesRegex(ValueError, 'Reload registry boundary'):
            self.check(meta=meta)

    def test_pending_real_contract_and_missing_reviewed_fingerprint_are_rejected(self):
        with self.assertRaisesRegex(ValueError, 'native attachment'):
            self.check(meta=pending_metadata())
        self.check()
        for fingerprint in (None, '', 'unreviewed', '1' * 63):
            meta = copy.deepcopy(self.meta)
            meta['injections'][0]['function_fingerprint'] = fingerprint
            with self.subTest(fingerprint=fingerprint), self.assertRaises(ValueError):
                self.check(meta=meta)

    def test_authority_package_contract_and_competing_owner_mutations_reject(self):
        self.check()
        for section, fields in [
            ('modules', ['module_id', 'file', 'name', 'visibility', 'mod_id', 'source']),
            ('injections', ['injection_id', 'target', 'file', 'function_fingerprint',
                            'kind', 'cancellable', 'bindings', 'mod_id', 'helper_source']),
        ]:
            for field in fields:
                plan = copy.deepcopy(self.plan)
                plan[section][0][field] = 'changed'
                with self.subTest(section=section, field=field), self.assertRaises(ValueError):
                    self.check(plan)
            for competing in (False, True):
                plan = copy.deepcopy(self.plan)
                if competing:
                    entry = dict(plan[section][0], mod_id='another_mod')
                    entry['module_id' if section == 'modules' else 'injection_id'] = 'alias'
                    plan[section].append(entry)
                else:
                    plan[section].clear()
                with self.subTest(section=section, competing=competing), self.assertRaises(ValueError):
                    self.check(plan)
        for relative in self.meta['official_source_sha256']:
            path = self.workspace / relative
            before = path.read_bytes()
            path.write_bytes(before + b'drift')
            with self.assertRaises(ValueError):
                self.check()
            path.write_bytes(before)
            plan = copy.deepcopy(self.plan)
            plan['inputs'][relative] = 'wrong authority'
            with self.assertRaises(ValueError):
                self.check(plan)
        for required in self.meta['modules'] + self.meta['injections']:
            path = self.helpers / required['source_file']
            before = path.read_bytes()
            path.write_bytes(before + b'drift')
            with self.assertRaises(ValueError):
                self.check()
            path.write_bytes(before)
        self.check()

    def test_state_owner_constructor_and_nonserialization_contracts_reject_drift(self):
        self.check()
        for field in self.meta['states'][0]:
            plan = copy.deepcopy(self.plan)
            plan['states'][0][field] = 'changed'
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.check(plan)
        for field in self.meta['states'][0]['constructors'][0]:
            plan = copy.deepcopy(self.plan)
            plan['states'][0]['constructors'][0][field] = 'changed'
            with self.subTest(constructor=field), self.assertRaises(ValueError):
                self.check(plan)
        for variant in ('missing', 'competing', 'foreign', 'extra-constructor', 'module', 'injection', 'bridge'):
            plan = copy.deepcopy(self.plan)
            if variant == 'missing':
                plan['states'].clear()
            elif variant == 'competing':
                plan['states'].append(dict(plan['states'][0], state_id='alias'))
            elif variant == 'foreign':
                plan['states'][0]['mod_id'] = 'another_mod'
            elif variant == 'extra-constructor':
                plan['states'].append({'state_id': 'alias', 'constructors': [{'file': GUARD.OWNER_FILE}]})
            else:
                section = {'module': 'modules', 'injection': 'injections', 'bridge': 'bridges'}[variant]
                plan.setdefault(section, []).append({'file': GUARD.OWNER_FILE})
            with self.subTest(variant=variant), self.assertRaises(ValueError):
                self.check(plan)
        for field in ('node', 'fingerprint', 'constructors', 'deserialize'):
            meta = copy.deepcopy(self.meta)
            meta['states'][0][field] = None
            with self.subTest(unreviewed=field), self.assertRaises((ValueError, TypeError)):
                self.check(meta=meta)

    def test_cache_entry_and_revision_hooks_require_exact_owners_constructors_and_payloads(self):
        self.check()
        for index in range(1, len(self.plan['states'])):
            for field in self.meta['states'][index]:
                plan = copy.deepcopy(self.plan)
                plan['states'][index][field] = 'changed'
                with self.subTest(state=index, field=field), self.assertRaises(ValueError):
                    self.check(plan)
            for field in self.meta['states'][index]['constructors'][0]:
                plan = copy.deepcopy(self.plan)
                plan['states'][index]['constructors'][0][field] = 'changed'
                with self.subTest(state=index, constructor=field), self.assertRaises(ValueError):
                    self.check(plan)
        for index in range(1, len(self.plan['injections'])):
            for field in self.plan['injections'][index]:
                plan = copy.deepcopy(self.plan)
                plan['injections'][index][field] = 'changed'
                with self.subTest(hook=index, field=field), self.assertRaises(ValueError):
                    self.check(plan)
        for section in ('states', 'injections'):
            plan = copy.deepcopy(self.plan)
            plan[section].pop()
            with self.subTest(missing=section), self.assertRaises(ValueError):
                self.check(plan)

    def test_direct_state_requires_exact_output_and_keeps_other_owner_checks(self):
        state = self.meta['states'][0]
        site, relative = state['state_id'], state['file']
        output = b'adapted owner with direct state\n'
        entry = {'id': site, 'file': relative,
                 'before': self.meta['official_source_sha256'][relative],
                 'after': GUARD.CONTRACT.digest(output)}
        (self.workspace / relative).write_bytes(output)
        self.plan['inputs'][relative] = entry['after']
        self.meta.update(direct_review_status='reviewed', horizon_revision='fixture-pin',
                         direct_attachments=[entry])
        receipt = {'horizon_revision': 'fixture-pin', 'attachments': [copy.deepcopy(entry)]}
        original = copy.deepcopy(self.plan)
        self.plan['states'] = [e for e in self.plan['states'] if e['state_id'] != site]
        def check(plan=None, proof=receipt):
            GUARD.check(self.plan if plan is None else plan, self.workspace, self.meta, self.helpers, proof)
        check()
        with self.assertRaisesRegex(ValueError, 'also owned'):
            check(original)
        changed = copy.deepcopy(self.plan)
        changed['states'][0]['ty'] = 'unreviewed'
        with self.assertRaisesRegex(ValueError, 'contract differs'):
            check(changed)
        self.meta['direct_review_status'] = 'pending'
        with self.assertRaisesRegex(ValueError, 'await combined'):
            check()

    def test_direct_receipt_cannot_omit_either_combined_guard(self):
        import json
        path = Path(self.temp.name) / 'direct-plan.json'
        proof = Path(self.temp.name) / 'direct-receipt.json'
        path.write_text(newline='\n', data=json.dumps({}), encoding='utf-8')
        proof.write_text(newline='\n', data=json.dumps({'attachments': []}), encoding='utf-8')
        for names in ('structures', 'block_entities', 'protection'):
            result = subprocess.run([sys.executable, str(ROOT / 'tools/check_sliced_plan.py'),
                str(path), str(self.workspace), names, str(proof)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
            self.assertIn('require the structures and block_entities guards', result.stderr)

    def test_public_host_reexports_cannot_escape_guard_by_renaming(self):
        import json
        plan = {'modules': [{'module_id': 'foreign-export', 'mod_id': 'foreign',
            'file': 'crates/horizon-client/src/client_lib.rs',
            'name': 'renamed_host_types', 'visibility': 'pub', 'source': 'pub use sunrise_world::nbt;'}]}
        self.assertTrue(GUARD.kept(plan))
        path = Path(self.temp.name) / 'reexport-plan.json'
        path.write_text(json.dumps(plan), encoding='utf-8', newline='\n')
        result = subprocess.run([sys.executable, str(ROOT / 'tools/check_sliced_plan.py'),
            str(path), str(self.workspace), 'protection'], capture_output=True, text=True)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn('requires the block_entities guard', result.stderr)

    def test_sliced_guard_omission_rejects_even_for_renamed_foreign_hook(self):
        import json
        for entry in [*self.plan['modules'],
                      *[dict(e, state_id='foreign-state', mod_id='foreign') for e in self.plan['states']],
                      *[dict(e, injection_id='foreign-hook', mod_id='foreign') for e in self.plan['injections']]]:
            section = 'modules' if 'module_id' in entry else 'states' if 'state_id' in entry else 'injections'
            plan = {section: [entry]}
            self.assertTrue(GUARD.kept(plan))
            path = Path(self.temp.name) / 'plan.json'
            path.write_text(json.dumps(plan), encoding='utf-8', newline='\n')
            result = subprocess.run([sys.executable, str(ROOT / 'tools/check_sliced_plan.py'),
                                     str(path), str(self.workspace), 'protection'],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
            self.assertIn('requires the block_entities guard', result.stderr)
        self.assertFalse(GUARD.kept({'injections': [{'injection_id': 'unrelated', 'target': 'unrelated'}]}))
