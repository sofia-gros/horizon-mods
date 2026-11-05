"""Structural adapters for the production tag showcase and its existing wire path."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class BookShowcaseTagWiring(unittest.TestCase):
    def test_world_tags_known_items_and_body_layout_are_wired(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        start = source.index('if item.get("tag").is_some()')
        branch = source[start:source.index('let id = super::tconstruct_materials_book_recipe::showcase_item(item)?;', start)]
        self.assertIn('game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags()', branch)
        self.assertIn('game.world.chunks.__horizon_mixins_tconstruct_melter_known_items()', branch)
        self.assertIn('showcase_tag(item, tags, known)', branch)
        self.assertEqual(source.count('graphic.get("showcase_items").is_some()'), 2)
        self.assertIn('graphic["body_runs"]', source)
        self.assertIn('remaining.unwrap_or_else(|| split_styled', source)
        self.assertIn('id == "tconstruct:creative_slot"', source)

    def test_hover_uses_same_selected_stack_and_keeps_text_for_empty_choices(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book-draw.rs').read_text()
        start = source.index('if spec.get("showcase_item").is_some()')
        branch = source[start:source.index('if let Some(icon) = spec.get("source_icon")', start)]
        self.assertEqual(branch.count('showcase_display_item(&spec, seconds)'), 1)
        self.assertIn('draw_stack(batch, id, 1, 0, None, x, y)', branch)
        self.assertIn('item_tooltip = Some(stack)', branch)
        self.assertIn('}\n        if let Some(content)', branch)
        self.assertIn('decode_runs', branch)


if __name__ == '__main__':
    unittest.main()


class SupportedBonusGuideWiring(unittest.TestCase):
    def test_bonus_pages_keep_numbered_inputs_anvil_and_bottle_return_notice(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        start = source.index('fn bonus_modifier_pages(')
        body = source[start:source.index('fn ordinary_modifier_pages(', start)]
        for text in ('entry.inputs.iter().enumerate()', 'Repeated ingredients need separate slots',
                     'entry.inputs.len() > 3', "More than three inputs require a Tinker",
                     'entry.returns_bottles', 'Each consumed {} returns one {}',
                     'item_name("minecraft:dragon_breath")', 'item_name("minecraft:glass_bottle")'):
            self.assertIn(text, body)

    def test_puny_bonus_group_reads_current_snapshots_and_existing_pagination(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        start = source.index('fn bonus_modifier_pages(')
        body = source[start:source.index('fn dynamic_materials(', start)]
        for accessor in ('modifier_resource_snapshot', 'recipe_snapshot', 'resolved_item_tags'):
            self.assertIn('__horizon_mixins_tconstruct_' + accessor, body)
        for call in ('bonus_guide_registry()', 'supported_guide_tag(raw, tag, &supported)',
                     'bonus_guide_recipes(recipes, &modifiers, &items, &tools',
                     'ingredient_matches(ingredient, item, tags)', 'split_styled(game,',
                     'tconstruct:book-title', 'partial guide'):
            self.assertIn(call, body)
        self.assertIn('puny && key == "slotless"', source)
        self.assertIn('Some("group_bonus_upgrades")', source)
        self.assertIn('/extraData/tconstruct:load_modifiers/tag', source)
        self.assertIn('chapter_pages.extend(bonus_pages)', source)
        self.assertIn('source_pages(game, &path, book)', (ROOT / 'mixins/tconstruct/source-materials-book-request.rs').read_text())

    def test_gameplay_bonus_registry_is_not_duplicated_for_the_book(self):
        rules = (ROOT / 'mixins/tconstruct/source-upgrade-rules.rs').read_text()
        self.assertIn("bonus_guide_registry() -> &'static [(&'static str, i32, i32)] { &BONUS_SLOTS }", rules)
        self.assertIn('pub(super) fn ingredient_matches(', rules)


class OrdinaryModifierGuideWiring(unittest.TestCase):
    def test_fluid_guides_use_fresh_stacks_and_runtime_repeatability(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        start = source.index('fn ordinary_modifier_pages(')
        body = source[start:source.index('fn dynamic_materials(', start)]
        for text in ('stack_ingredient_matches(ingredient, &ItemStack::new(item, 1), tags)',
                     'fluid_guide_policy', 'entry.repeatable_maximum',
                     'entry.empty_inputs[index]', 'fresh empty container',
                     'Filled or otherwise modified containers may be rejected',
                     'More than three inputs require a Tinker'):
            self.assertIn(text, body)
        rules = (ROOT / 'mixins/tconstruct/source-upgrade-rules.rs').read_text()
        self.assertIn('let maximum = repeatable_fluid_limit(id)?;', rules)
        self.assertIn('let fluid_limit = repeatable_fluid_limit(id);', rules)

    def test_source_recipes_and_mighty_group_metadata_are_pinned(self):
        import hashlib
        import json
        import zipfile
        fixture = json.loads((ROOT / 'tests/fixtures/ordinary-modifier-guides.json').read_text())
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            for path, recipe in {**fixture['recipes'], **fixture.get('incremental_recipes', {}), **fixture.get('bulk_incremental_recipes', {}), **fixture.get('fluid_recipes', {}), **fixture.get('bonus_recipes', {})}.items():
                self.assertEqual(recipe, json.loads(archive.read(path)), path)
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-modifiers.zip') as archive:
            for path, tag in fixture['modifier_tags'].items():
                self.assertEqual(tag, json.loads(archive.read(path)), path)
        section = ROOT / 'content/assets/tconstruct/book/mighty_smelting/sections/abilities.json'
        self.assertEqual(hashlib.sha256(section.read_bytes()).hexdigest(), 'ae05346a54e7665a09dd4ab6fd1d71628b2bc661d4f2fcf1cc062945ea74cac7')
        groups = json.loads(section.read_text())
        self.assertEqual(len(groups), 11)
        self.assertEqual(groups[1]['extraData']['tconstruct:load_modifiers']['tag'], 'tconstruct:abilities/interaction')
        self.assertEqual(groups[9]['extraData']['tconstruct:load_modifiers']['tag'], 'tconstruct:abilities/armor/boots')

    def test_generated_recipe_pages_use_station_gates_and_separate_inputs(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        start = source.index('fn ordinary_modifier_pages(')
        body = source[start:source.index('fn dynamic_materials(', start)]
        for text in ('ordinary_guide_registry()', 'supported_guide_tag(raw, tag, &supported)',
                     'ordinary_guide_recipes_with_fluids(recipes, &modifiers, &items, &tools',
                     'ingredient_matches(ingredient, item, tags)', 'tool_supported_with_tags',
                     'entry.inputs.iter().enumerate()', 'entry.check_trait_level',
                     'entry.upgrade_cost, entry.ability_cost', 'split_styled(game,', 'partial guide'):
            self.assertIn(text, body)
        self.assertIn('(puny && key == "upgrades") || (mighty && key == "abilities")', source)
        self.assertIn('if mighty { continue; }', source)
        self.assertIn('ordinary_modifier_pages(game, &ordinary_tags)', source)
        rules = (ROOT / 'mixins/tconstruct/source-upgrade-rules.rs').read_text()
        self.assertIn('if !ordinary_modifier_supported(id) { continue; }', rules)
        self.assertIn('if !tool_supported_with_tags(id, &tool.item, tags) { continue; }', rules)
        self.assertIn('tool_supported_with_tags(modifier, item, tags)', body)
        self.assertIn('ordinary_modifier_supported(id) && provisional_upgrade_enabled(id)', rules)

    def test_actual_station_support_gates_exclude_incremental_and_disabled_ids(self):
        import shutil
        import subprocess
        import tempfile
        if not shutil.which('rustc'):
            self.skipTest('rustc required for the actual Station support gate fixture')
        source = (ROOT / 'mixins/tconstruct/source-upgrade-rules.rs').read_text()
        def function(name):
            start = source.index('fn ' + name + '(')
            opening = source.index('{', start)
            depth = 1
            end = opening + 1
            while depth:
                depth += (source[end] == '{') - (source[end] == '}')
                end += 1
            return source[start:end]
        chunks = ['use std::collections::{HashMap, HashSet};']
        for name in ('UPGRADES', 'BONUS_SLOTS'):
            start = source.index('const ' + name + ':')
            chunks.append(source[start:source.index('];', start) + 2])
        chunks.extend(function(name) for name in ('ordinary_modifier_supported', 'ordinary_guide_registry',
                                                'provisional_upgrade_enabled', 'supported', 'tool_supported_with_tags'))
        chunks.append(r'''#[test]
fn actual_guide_gate() {
    let ids = ordinary_guide_registry();
    for id in ["tconstruct:diamond", "tconstruct:emerald", "tconstruct:netherite", "tconstruct:glowing"] {
        assert!(ids.contains(&id.to_owned()));
        assert!(ordinary_modifier_supported(id));
    }
    for id in ["tconstruct:haste", "tconstruct:sharpness", "tconstruct:unbreakable", "tconstruct:reinforced", "tconstruct:writable"] {
        assert!(!ids.contains(&id.to_owned()));
    }
    assert!(tool_supported_with_tags("tconstruct:step_up", "tconstruct:travelers_leggings", &HashMap::new()));
    assert!(!tool_supported_with_tags("tconstruct:step_up", "tconstruct:pickaxe", &HashMap::new()));
    assert!(tool_supported_with_tags("tconstruct:glowing", "tconstruct:pickaxe", &HashMap::new()));
    assert!(!tool_supported_with_tags("unknown:modifier", "tconstruct:pickaxe", &HashMap::new()));
    let mut tags = HashMap::from([("tconstruct:modifiable/interactable/charge".into(), HashSet::from(["tconstruct:pickaxe".into()]))]);
    // An overridden recipe may select helmets, but this shared gate still rejects them.
    assert!(!tool_supported_with_tags("tconstruct:slurping", "tconstruct:travelers_helmet", &tags));
    assert!(tool_supported_with_tags("tconstruct:slurping", "tconstruct:pickaxe", &tags));
    tags.get_mut("tconstruct:modifiable/interactable/charge").unwrap().insert("tconstruct:travelers_helmet".into());
    assert!(tool_supported_with_tags("tconstruct:slurping", "tconstruct:travelers_helmet", &tags));
    tags.remove("tconstruct:modifiable/interactable/charge");
    assert!(!tool_supported_with_tags("tconstruct:slurping", "tconstruct:travelers_helmet", &tags));
}
''')
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'guide_gate.rs'
            path.write_text('\n'.join(chunks))
            binary = Path(directory) / 'guide_gate'
            subprocess.run(['rustc', '--edition=2021', '--test', str(path), '-o', str(binary)], check=True, capture_output=True, text=True)
            subprocess.run([str(binary)], check=True, capture_output=True, text=True)


class IncrementalModifierGuideWiring(unittest.TestCase):
    def test_partial_incremental_pages_use_current_snapshots_and_shared_limits(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        start = source.index('fn incremental_modifier_pages(')
        body = source[start:source.index('fn dynamic_materials(', start)]
        for text in ('modifier_resource_snapshot()', 'recipe_snapshot()', 'resolved_item_tags()',
                     'incremental_guide_registry()', 'supported_guide_tag(raw, tag, &supported)',
                     'incremental_guide_recipes(recipes, &modifiers, &registry, &items, &tools',
                     'tool_supported_with_tags(modifier, item, tags)', 'incremental_limits_supported',
                     'entry.per_item, entry.needed, entry.maximum, entry.upgrade_cost', 'entry.leftover', 'unused progress points', 'split_styled(game,', 'partial guide'):
            self.assertIn(text, body)
        self.assertIn('if puny && key == "upgrades" { chapter_pages.extend(incremental_modifier_pages', source)
        rules = (ROOT / 'mixins/tconstruct/source-upgrade-rules.rs').read_text()
        self.assertIn('if !incremental_limits_supported(id, needed, max) { continue; }', rules)
        self.assertIn('*needed > 1 && provisional_upgrade_enabled(id)', rules)
