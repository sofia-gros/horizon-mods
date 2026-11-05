use horizon_client::advancements::Tree;
use std::sync::Arc;

pub(super) fn apply(game: &mut super::Game<'_>, tree: Arc<Tree>) -> Arc<Tree> {
    if game.remote.is_some() { return tree; }
    let generation = *game.world.chunks.__horizon_mixins_tconstruct_source_recipe_generation();
    if game.adv.__horizon_mixins_tconstruct_fake_material_discovery_cache().as_ref()
        .is_some_and(|(built, previous)| *built == generation && Arc::ptr_eq(previous, &tree)) {
        return tree;
    }
    let mut variants: [Vec<String>; 2] = Default::default();
    // Only a completed source capture has validated reserved index entries.
    if game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().is_some()
        && let Some(tables) = game.world_tables.as_ref() {
        for id in tables.stations.recipe_index.ids() {
            let family = if id.starts_with("tconstruct:fake_metal_native/block/") { 0 }
                else if id.starts_with("tconstruct:fake_metal_native/ingots/") { 1 }
                else { continue; };
            variants[family].push(id.clone());
        }
    }
    let Some(next) = horizon_client::advancements::tconstruct_source_fake_material_discovery::remap(&tree, &variants)
        else {
            *game.adv.__horizon_mixins_tconstruct_fake_material_discovery_cache_mut() = Some((generation, tree.clone()));
            return tree;
        };
    let next = Arc::new(next);
    game.adv.tree = next.clone();
    *game.adv.__horizon_mixins_tconstruct_fake_material_discovery_cache_mut() = Some((generation, next.clone()));
    game.tconstruct_invalidate_advancement_inventory_index();
    next
}

pub(super) fn refresh(game: &mut super::Game<'_>) {
    // Do not race/replace an as-yet-unconsumed prefetched tree. Its normal
    // advancement_tree return hook will map it against the ready world index.
    if !game.adv.tree.list.is_empty() {
        let tree = game.adv.tree.clone();
        apply(game, tree);
    }
}
