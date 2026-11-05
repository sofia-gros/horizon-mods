use super::{Game, tconstruct_fortune_harvest as rules};
use horizon_items::items::ItemStack;

// This is a loot-context copy only. Never write synthetic enchantments into
// the held inventory stack, its saved data, or the client-visible item.
fn projected_tool(
    game: &Game<'_>,
    held: Option<&ItemStack>,
    include_worn: bool,
) -> Option<ItemStack> {
    if game.remote.is_some() || game.player.is_none() {
        return None;
    }
    let stack = held?;
    if !stack.item.starts_with("tconstruct:") || stack.count != 1 || stack.wire_patch().is_some() {
        return None;
    }
    super::tconstruct_source_melee_shape::intact(stack)?;
    let tags = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_resolved_item_tags()
        .as_ref()?
        .as_ref()
        .ok()?;
    if !tags
        .get("tconstruct:modifiable/harvest")?
        .contains(stack.item.as_ref())
    {
        return None;
    }
    let raw = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()?
        .as_ref()
        .ok()?;
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    let mut levels = [0; 2];
    for (index, id) in ["tconstruct:fortune", "tconstruct:fortunate"]
        .into_iter()
        .enumerate()
    {
        let level =
            super::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, id)?;
        if level > 0 {
            let path = format!(
                "tconstruct:tinkering/modifiers/{}",
                id.strip_prefix("tconstruct:")?
            );
            if !rules::known(
                id,
                super::tconstruct_modifier_snapshot::highest_definition(raw, &path),
            ) {
                return None;
            }
            levels[index] = level;
        }
    }
    let mut worn_levels = [0; 4];
    if include_worn && !game.creative {
        let worn = tags.get("tconstruct:modifiable/armor/worn")?;
        let modifiable = tags.get("tconstruct:modifiable")?;
        for (index, armor) in game.armor.iter().enumerate() {
            let Some(armor) = armor else {
                continue;
            };
            if rules::armor_slot(&armor.item) != Some(index)
                || !worn.contains(armor.item.as_ref())
                || !modifiable.contains(armor.item.as_ref())
            {
                continue;
            }
            if armor.count != 1 || armor.wire_patch().is_some() {
                return None;
            }
            // Unlike the armor Looting iterator, HarvestEnchantments uses
            // getValidTool directly and does not suppress broken armor.
            let traits = super::tconstruct_source_material_traits::levels(&game.world, armor)?;
            let level = super::tconstruct_source_material_traits::level_with_upgrade(
                armor,
                &traits,
                "tconstruct:fortune",
            )?;
            if level > 0
                && !rules::known(
                    "tconstruct:fortune",
                    super::tconstruct_modifier_snapshot::highest_definition(
                        raw,
                        "tconstruct:tinkering/modifiers/fortune",
                    ),
                )
            {
                return None;
            }
            worn_levels[index] = level;
        }
    }
    if levels == [0, 0] && worn_levels == [0; 4] {
        return None;
    }
    // Native reads the first entry. Reject ambiguous duplicate component keys
    // rather than computing a different value from later duplicate entries.
    let mut seen = std::collections::HashSet::new();
    if stack
        .enchantments()
        .iter()
        .any(|(id, _)| !seen.insert(id.as_ref()))
    {
        return None;
    }
    let level = rules::level(
        stack.enchantment_level("minecraft:fortune"),
        levels[0],
        levels[1],
    )?;
    let level = rules::with_worn(level, &worn_levels)?;
    let mut projected = stack.clone();
    projected.enchant("minecraft:fortune", level);
    Some(projected)
}

pub(super) fn drops(
    game: &mut Game<'_>,
    pos: [i32; 3],
    state: horizon_world::registry::BlockStateId,
    held: Option<&ItemStack>,
    block_entity: &Option<horizon_world::nbt::Tag>,
) -> std::ops::ControlFlow<Vec<ItemStack>> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(projected) = projected_tool(game, held, true) else {
        return Continue(());
    };
    Break(roll(
        game,
        pos,
        state,
        Some(&projected),
        block_entity.clone(),
    ))
}

// Source HarvestModule reads Block.getDrops directly and never runs the worn
// harvest hook. Keep constant tool enchantments, but do not grant armor Fortune
// merely because the crop adapter previously shared block_loot_at.
pub(super) fn crop_drops(
    game: &mut Game<'_>,
    pos: [i32; 3],
    state: horizon_world::registry::BlockStateId,
    held: Option<&ItemStack>,
    block_entity: Option<horizon_world::nbt::Tag>,
) -> Vec<ItemStack> {
    let projected = projected_tool(game, held, false);
    roll(game, pos, state, projected.as_ref().or(held), block_entity)
}

fn roll(
    game: &mut Game<'_>,
    pos: [i32; 3],
    state: horizon_world::registry::BlockStateId,
    held: Option<&ItemStack>,
    block_entity: Option<horizon_world::nbt::Tag>,
) -> Vec<ItemStack> {
    // Match Game::block_loot_at's native roll exactly, changing only the tool
    // supplied to it. Keep the named random sequence, dynamic drops, world
    // location, block entity, and native loot callback. There is one roll, not
    // a post-drop count multiplier or a second random reroll.
    let registry = game.pack.assets.registry;
    let dynamic = game.pending.dynamic(pos);
    let place = game.loot_place_at(pos.map(|v| f64::from(v) + 0.5), block_entity);
    let items = match &game.world_tables {
        Some(tables) => &tables.items,
        None => game.res.items(),
    };
    let mut drops = horizon_world::loot::with_native_loot_place(place, || {
        match items.block_loot_sequence(registry, state) {
            Some(sequence) => {
                let random = game.rand_sequences.get(&sequence, game.level.seed);
                items.block_drops_by_player(registry, state, held, &dynamic, random)
            }
            None => {
                items.block_drops_by_player(registry, state, held, &dynamic, &mut game.world.random)
            }
        }
    });
    let player = game.native_acting_id();
    game.native_block_drops(pos, state, Some(player), held, None, &mut drops);
    drops
}
