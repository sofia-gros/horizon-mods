"""Exercise actual package publication and forbidden-input rejection, not prose."""
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch
import sys
import zipfile

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('clean_port', ROOT / 'tools/port.py')
port = importlib.util.module_from_spec(spec)
spec.loader.exec_module(port)


class CleanPort(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        # Resolved: on Windows runners the temp dir is a short 8.3 path that resolve() expands.
        self.root = Path(self.temp.name).resolve()
        for rel in ['docs/port-plan.json', 'tconstruct/compatibility.json']:
            dst = self.root / rel
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(ROOT / rel, dst)
        shutil.copytree(ROOT / port.BUNDLE, self.root / port.BUNDLE)

    def test_package_contains_only_checked_product_and_never_overwrites(self):
        output = self.root / '.local/test.hnmod'
        (self.root / 'legacy').mkdir()
        (self.root / 'legacy/old.rs').write_text(newline='\n', data='must not ship', encoding='utf-8')
        digest = port.package(self.root, output)
        with zipfile.ZipFile(output) as archive:
            self.assertEqual(set(archive.namelist()), {
                'horizon-native.json', 'mixins/tconstruct_native/horizon-mixin.toml',
                'mixins/tconstruct_native/api/Cargo.toml', 'mixins/tconstruct_native/api/src/lib.rs',
                'mixins/tconstruct_native/api/src/materials.rs',
                'mixins/tconstruct_native/api/src/material_traits.rs',
                'mixins/tconstruct_native/api/src/modifier_entries.rs',
                'mixins/tconstruct_native/api/src/modifier_module_types.rs',
                'mixins/tconstruct_native/api/src/formula.rs',
                'mixins/tconstruct_native/api/src/material_recipes.rs',
                'mixins/tconstruct_native/api/src/modifier_recipes.rs',
                'mixins/tconstruct_native/api/src/modifier_removal.rs',
                'mixins/tconstruct_native/api/src/overslime.rs',
                'mixins/tconstruct_native/api/src/worktable_recipes.rs',
                'mixins/tconstruct_native/api/src/part_builder_recipes.rs',
                'mixins/tconstruct_native/api/src/tool_predicate.rs',
                'mixins/tconstruct_native/api/src/modifier_modules.rs',
                'mixins/tconstruct_native/api/src/modifiers.rs',
                'mixins/tconstruct_native/api/src/tool_rebuild.rs',
                'mixins/tconstruct_native/api/src/tool_stack.rs',
                'mixins/tconstruct_native/api/src/tool_damage.rs',
                'mixins/tconstruct_native/api/src/tool_repair.rs',
                'mixins/tconstruct_native/api/src/tool_swap.rs',
                'mixins/tconstruct_native/api/src/tool_build.rs',
                'mixins/tconstruct_native/api/src/tool_parts.rs',
                'mixins/tconstruct_native/api/src/armor_textures.rs',
                'mixins/tconstruct_native/api/src/armor_worn.rs',
                'mixins/tconstruct_native/api/src/tool_mining.rs',
                'mixins/tconstruct_native/api/src/mining_speed.rs',
                'mixins/tconstruct_native/api/src/break_speed_hook.rs',
                'mixins/tconstruct_native/api/src/melee_damage.rs',
                'mixins/tconstruct_native/api/src/entity_facts.rs',
                'mixins/tconstruct_native/api/src/knockback.rs',
                'mixins/tconstruct_native/api/src/melee_hook.rs',
                'mixins/tconstruct_native/api/src/conditional_stat.rs',
                'mixins/tconstruct_native/api/src/protection.rs',
                'mixins/tconstruct_native/api/src/protection_hook.rs',
                'mixins/tconstruct_native/api/src/tool_damage_hook.rs',
                'mixins/tconstruct_native/api/src/weapon_effect.rs',
                'mixins/tconstruct_native/api/src/thorns.rs',
                'mixins/tconstruct_native/api/src/hit_actions.rs',
                'mixins/tconstruct_native/api/src/capacity.rs',
                'mixins/tconstruct_native/api/src/effect_immunity.rs',
                'mixins/tconstruct_native/api/src/tool_inventory.rs',
                'mixins/tconstruct_native/api/src/effect_immunity_hook.rs',
                'mixins/tconstruct_native/api/src/unequip_hook.rs',
                'mixins/tconstruct_native/api/src/max_armor.rs',
                'mixins/tconstruct_native/api/src/max_armor_hook.rs',
                'mixins/tconstruct_native/api/src/wearer_modules.rs',
                'mixins/tconstruct_native/api/src/mob_disguise.rs',
                'mixins/tconstruct_native/api/src/mob_disguise_hook.rs',
                'mixins/tconstruct_native/api/src/edible.rs',
                'mixins/tconstruct_native/api/src/edible_hook.rs',
                'mixins/tconstruct_native/api/src/weapon_effect_hook.rs',
                'mixins/tconstruct_native/api/src/block_break_hook.rs',
                'mixins/tconstruct_native/api/src/damage_tags.rs',
                'mixins/tconstruct_native/api/src/damage_vanilla.rs',
                'mixins/tconstruct_native/api/src/part_recycling.rs',
                'mixins/tconstruct_native/api/src/tool_actions.rs',
                'mixins/tconstruct_native/api/src/tool_action_hook.rs',
                'mixins/tconstruct_native/api/src/tool_slot.rs',
                'mixins/tconstruct_native/api/src/tool_station.rs',
                'mixins/tconstruct_native/api/src/station_menu.rs',
                'mixins/tconstruct_native/api/src/part_builder_menu.rs',
                'mixins/tconstruct_native/api/src/worktable_menu.rs',
                'mixins/tconstruct_native/api/src/creative_parts.rs',
                'mixins/tconstruct_native/api/src/creative_tools.rs',
                'mixins/tconstruct_native/api/src/creative.rs',
                'mixins/tconstruct_native/api/src/material_stats.rs',
                'mixins/tconstruct_native/api/src/material_snapshot.rs',
                'mixins/tconstruct_native/api/src/material_sync.rs',
                'mixins/tconstruct_native/api/src/model_flags.rs',
                'mixins/tconstruct_native/api/src/tool_materials.rs',
                'mixins/tconstruct_native/api/src/tool_stats.rs',
                'mixins/tconstruct_native/api/src/tool_definitions.rs',
                'mixins/tconstruct_native/api/src/common.rs',
                'mixins/tconstruct_native/api/src/config.rs',
                'mixins/tconstruct_native/api/src/conditions.rs',
                'mixins/tconstruct_native/api/src/tables/mod.rs',
                'mixins/tconstruct_native/api/src/tables/lazy_result.rs',
                'mixins/tconstruct_native/api/src/tables/input_consumption.rs',
                'mixins/tconstruct_native/api/src/tables/recipe_selection.rs',
                'mixins/tconstruct_native/api/src/tables/station.rs',
                'mixins/tconstruct_native/LICENSE'})
            native = json.loads(archive.read('horizon-native.json'))
            self.assertEqual(native['id'], 'tconstruct_native')
            manifest = port.tomllib.loads(archive.read('mixins/tconstruct_native/horizon-mixin.toml').decode())
            self.assertEqual(manifest['dependencies'], {'pluto_lib': '^0.1'})
            self.assertEqual(manifest['mod']['id'], native['id'])
        self.assertEqual(digest, port.checked_files(self.root)[3])
        before = output.read_bytes()
        with self.assertRaises(ValueError):
            port.package(self.root, output)
        self.assertEqual(output.read_bytes(), before)
        with self.assertRaises(ValueError):
            port.package(self.root, self.root / 'tconstruct/overwrite.hnmod')

    def test_tool_stack_table_matches_the_declared_tool_items(self):
        import re
        source = (ROOT / 'tconstruct/mixins/tconstruct/api/src/tool_parts.rs').read_text(encoding='utf-8')
        table = dict((name, int(size)) for name, size in re.findall(r'\("([a-z_]+)", (\d+)\)', source.split('TOOL_STACKS')[1].split('];')[0]))
        declared = {}
        for path in (ROOT / port.CONTENT / 'data/tconstruct/horizon/item').glob('*.json'):
            item = json.loads(path.read_text(encoding='utf-8'))
            size = item.get('max_stack_size')
            if size is not None:
                declared[path.stem] = size
        self.assertEqual(table, declared)

    def test_content_mod_packages_only_reviewed_declarations(self):
        shutil.copytree(ROOT / port.CONTENT, self.root / port.CONTENT)
        _, manifest, _, _ = port.checked_files(self.root)
        version = manifest['mod']['version']
        content, digest = port.checked_content(self.root, version)
        self.assertIn('horizon-mod.toml', content)
        self.assertTrue(any(name.startswith('data/tconstruct/horizon/item/') for name in content))
        output = self.root / '.local/test.hmod'
        self.assertEqual(port.package_content(self.root, output), digest)
        with zipfile.ZipFile(output) as archive:
            self.assertEqual(set(archive.namelist()), set(content))
        with self.assertRaises(ValueError):
            port.package_content(self.root, output)  # packages never overwrite
        item = self.root / port.CONTENT / 'data/tconstruct/horizon/item/pick_head.json'
        for bad in ({'template': 'tconstruct:pick_head'}, {'template': 'minecraft:flint', 'script': 'x'}, []):
            item.write_text(newline='\n', data=json.dumps(bad), encoding='utf-8')
            with self.assertRaises(ValueError):
                port.checked_content(self.root, version)
        item.write_text(newline='\n', data=json.dumps({'template': 'minecraft:flint'}), encoding='utf-8')
        stray = self.root / port.CONTENT / 'data/tconstruct/horizon/entity_type/slime.json'
        stray.parent.mkdir(parents=True)
        stray.write_text(newline='\n', data='{}', encoding='utf-8')
        with self.assertRaises(ValueError):
            port.checked_content(self.root, version)  # a file with no reviewed shape
        stray.unlink()
        menu = self.root / port.CONTENT / 'data/tconstruct/horizon/menu/tinker_station.json'
        for bad in ({'slots': 6, 'results': [6]}, {'slots': 0}, {'slots': 6, 'script': 1}):
            menu.write_text(newline='\n', data=json.dumps(bad), encoding='utf-8')
            with self.assertRaises(ValueError):
                port.checked_content(self.root, version)
        menu.write_text(newline='\n', data=json.dumps({'slots': 6, 'data_slots': 2, 'results': [5]}), encoding='utf-8')
        block = self.root / port.CONTENT / 'data/tconstruct/horizon/block/tinker_station.json'
        for bad in ({'template': 'tconstruct:x'}, {'template': 'minecraft:stone', 'properties': 3}, {'template': 'minecraft:stone', 'script': 1}):
            block.write_text(newline='\n', data=json.dumps(bad), encoding='utf-8')
            with self.assertRaises(ValueError):
                port.checked_content(self.root, version)
        block.write_text(newline='\n', data=json.dumps({'template': 'minecraft:stone'}), encoding='utf-8')
        with self.assertRaises(ValueError):
            port.checked_content(self.root, '9.9.9')  # manifest version must match the native product

    def test_consumer_projection_requires_success_and_preserves_checked_sources(self):
        sys.path.insert(0, str(ROOT / 'tools'))
        try:
            import check_clean_linkage as linkage
        finally:
            sys.path.pop(0)
        prepared = self.root / '.local/clean-port/fixture'
        stage = prepared / 'applied'
        stage.mkdir(parents=True)
        workspace = stage / 'Cargo.toml'
        workspace.write_text(newline='\n', data='[workspace]\nmembers = [\n]\n', encoding='utf-8')
        receipt = prepared / 'api-check.json'
        receipt.write_text(newline='\n', data=json.dumps({'exit': 101, 'package': 'mod-pluto_lib'}))
        with patch.object(linkage, 'ROOT', self.root):
            with self.assertRaisesRegex(ValueError, 'successful PlutoLib'):
                linkage.prepare(prepared)
            self.assertFalse((stage / 'native-api').exists())
            receipt.write_text(newline='\n', data=json.dumps({'exit': 0, 'package': 'mod-pluto_lib'}))
            _, hashes = linkage.prepare(prepared)
            files = port.checked_files(self.root)[0]
            consumer = stage / 'native-api/tconstruct'
            self.assertEqual((consumer / 'src/lib.rs').read_bytes(), files['api/src/lib.rs'])
            projected = port.tomllib.loads((consumer / 'Cargo.toml').read_text())
            self.assertEqual(projected['dependencies']['serde_json'], '=1.0.154')
            self.assertEqual(projected['dependencies']['toml'], '=0.8.23')
            self.assertEqual(projected['dependencies']['pluto_lib'],
                {'package': 'mod-pluto_lib', 'path': '../pluto_lib'})
            self.assertEqual(projected['dependencies']['horizon-client'],
                {'path': '../../crates/horizon-client', 'default-features': False})
            self.assertEqual(set(hashes), set(files))
            before = workspace.read_bytes()
            with self.assertRaisesRegex(ValueError, 'already staged'):
                linkage.prepare(prepared)
            self.assertEqual(workspace.read_bytes(), before)

    def test_game_entry_projection_preserves_official_features_and_refuses_reuse(self):
        import hashlib
        sys.path.insert(0, str(ROOT / 'tools'))
        try:
            import check_game_entry as entry
        finally:
            sys.path.pop(0)
        prepared = self.root / '.local/clean-port/entry'
        stage = prepared / 'applied'
        for crate in ('horizon-client', 'horizon-official'):
            folder = stage / 'crates' / crate
            folder.mkdir(parents=True)
            (folder / 'Cargo.toml').write_text(newline='\n', data='[build-dependencies]\nhorizon-mod-api-gen = { path = "../horizon-mod-api-gen" }\nwinresource = "0.1"\n')
        workspace = stage / 'Cargo.toml'
        workspace.write_text(newline='\n', data='[workspace]\nmembers = [\n]\n')
        (prepared / 'direct-host.json').write_text(newline='\n', data=json.dumps({'horizon_revision': entry.PIN}))
        for name in ('native-check', 'api-check'):
            (prepared / (name + '.json')).write_text(newline='\n', data='{"exit": 0}')
        receipt = prepared / 'consumer-check.json'
        hashes = {name: hashlib.sha256(data).hexdigest() for name, data in port.checked_files(self.root)[0].items()}
        receipt.write_text(newline='\n', data=json.dumps({'exit': 1, 'consumer_inputs': hashes}))
        with patch.object(entry, 'ROOT', self.root):
            with self.assertRaisesRegex(ValueError, 'successful host/API/consumer'):
                entry.prepare(prepared)
            self.assertFalse((stage / 'crates/native-game').exists())
            receipt.write_text(newline='\n', data=json.dumps({'exit': 0, 'consumer_inputs': {}}))
            with self.assertRaisesRegex(ValueError, 'consumer inputs changed'):
                entry.prepare(prepared)
            receipt.write_text(newline='\n', data=json.dumps({'exit': 0, 'consumer_inputs': hashes}))
            entry.prepare(prepared)
            config = port.tomllib.loads((stage / 'crates/native-game/Cargo.toml').read_text())
            self.assertEqual(config['features']['default'], ['horizon-client/default', 'official'])
            self.assertFalse(config['dependencies']['horizon-client']['default-features'])
            self.assertEqual(config['dependencies']['mod_1']['package'], 'mod-tconstruct')
            self.assertEqual(config['package']['build'], '../horizon-client/build.rs')
            self.assertIn('crates/native-game', port.tomllib.loads(workspace.read_text())['workspace']['members'])
            before = workspace.read_bytes()
            with self.assertRaisesRegex(ValueError, 'already exists'):
                entry.prepare(prepared)
            self.assertEqual(workspace.read_bytes(), before)

    def test_forbidden_inputs_are_rejected_before_package_publication(self):
        base = self.root / port.BUNDLE
        manifest = base / 'horizon-mixin.toml'
        cargo = base / 'api/Cargo.toml'
        source = base / 'api/src/lib.rs'
        changes = [
            (manifest, '\n[[injection]]\nid="escape"\n'),
            (cargo, '\nother-host = { package = "horizon-client", version = "1" }\n'),
            (source, '\nuse horizon_client::game::Game;\n'),
            (source, '\ninclude!("../../../../legacy/old.rs");\n'),
        ]
        for path, addition in changes:
            with self.subTest(path=path.name, addition=addition):
                original = path.read_bytes()
                path.write_bytes(original + addition.encode())
                output = self.root / '.local/refused.hnmod'
                with self.assertRaises(ValueError):
                    port.package(self.root, output)
                self.assertFalse(output.exists())
                path.write_bytes(original)
        (base / 'orphan.rs').write_text(newline='\n', data='fn hidden() {}', encoding='utf-8')
        with self.assertRaises(ValueError):
            port.checked_files(self.root)

    def test_release_prerequisites_reject_incomplete_or_missing_final_review(self):
        path = self.root / 'docs/port-plan.json'
        data = json.loads(path.read_text(encoding='utf-8'))
        for task in data['tasks']:
            task.update(status='pending', evidence=[])
        path.write_text(json.dumps(data), encoding='utf-8', newline='\n')
        with self.assertRaisesRegex(ValueError, 'unfinished tasks:.*R1'):
            port.release_preflight(self.root)
        # Synthetic completed queue tests prerequisite logic, not game parity.
        for task in data['tasks']:
            task.update(status='done', evidence=[{'stage': 'game',
                'input_digest': 'fixture', 'scenario': 'fixture'}])
        path.write_text(newline='\n', data=json.dumps(data), encoding='utf-8')
        self.assertEqual(port.release_preflight(self.root), port.checked_files(self.root)[3])
        final = next(task for task in data['tasks'] if task['id'] == 'R5')
        final['evidence'] = [{'stage': 'pure', 'result': 'not runtime evidence'}]
        path.write_text(newline='\n', data=json.dumps(data), encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'gameplay completion'):
            port.release_preflight(self.root)
        data['tasks'].remove(final)
        path.write_text(newline='\n', data=json.dumps(data), encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'whole-product R5'):
            port.release_preflight(self.root)

    def test_queue_continues_independent_work_and_rejects_false_completion(self):
        path = self.root / 'docs/port-plan.json'
        data = json.loads(path.read_text(encoding='utf-8'))
        for task in data['tasks']:
            task.update(status='pending', evidence=[])
        data['tasks'][0].update(status='done', evidence=[{'stage': 'package', 'result': 'fixture'}])
        data['tasks'][1].update(status='blocked', blocker='native authorization not granted')
        mapping = next(t for t in data['tasks'] if t['id'] == 'R3')
        mapping.update(status='pending', evidence=[])
        path.write_text(newline='\n', data=json.dumps(data), encoding='utf-8')
        self.assertEqual(port.next_task(self.root)['id'], 'R3')
        data['tasks'][1].update(status='done', evidence=[{'stage': 'source', 'result': 'fixture'}])
        mapping.update(status='done', evidence=[{'stage': 'source', 'result': 'fixture'}])
        data['tasks'][2].update(status='done', evidence=[{'stage': 'pure', 'result': 'not a game run'}])
        path.write_text(newline='\n', data=json.dumps(data), encoding='utf-8')
        with self.assertRaises(ValueError):
            port.plan(self.root)
        data['tasks'][2]['status'] = 'pending'
        data['tasks'][0]['depends_on'] = ['R3']
        path.write_text(newline='\n', data=json.dumps(data), encoding='utf-8')
        with self.assertRaises(ValueError):
            port.plan(self.root)


if __name__ == '__main__':
    unittest.main()
