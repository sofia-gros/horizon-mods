use super::{Game, tconstruct_chrysophilite as rules};
use horizon_core::random::RandomSource;
use horizon_items::items::ItemStack;
use std::collections::HashSet;

pub(super) struct Context {
    gold: u8,
    ore_items: HashSet<String>,
}

pub(super) fn context(
    game: &Game<'_>,
    state: horizon_world::registry::BlockStateId,
) -> Option<Context> {
    if game.remote.is_some() || game.player.is_none() {
        return None;
    }
    let chunks = &game.world.chunks;
    let loot = chunks
        .__horizon_mixins_tconstruct_chrysophilite_loot_snapshot()
        .as_ref()?
        .as_ref()
        .ok()?;
    if !rules::enabled(loot) {
        return None;
    }
    let blocks = chunks
        .__horizon_mixins_tconstruct_resolved_block_tags()
        .as_ref()?
        .as_ref()
        .ok()?;
    let block = game.res.facts().states.get(state.0 as usize)?;
    if !blocks
        .get("tconstruct:chrysophilite_ores")?
        .contains(block.name.as_ref())
    {
        return None;
    }
    let tags = chunks
        .__horizon_mixins_tconstruct_resolved_item_tags()
        .as_ref()?
        .as_ref()
        .ok()?;
    let ores = tags.get("tconstruct:chrysophilite_ores")?;
    let worn = tags.get("tconstruct:modifiable/armor/worn")?;
    let modifiable = tags.get("tconstruct:modifiable")?;
    let raw = chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()?
        .as_ref()
        .ok()?;
    let native_gold = game
        .res
        .items()
        .item_tags
        .get("minecraft:piglin_safe_armor");
    let mut active = false;
    let mut gold = 0;
    for (slot, stack) in game
        .armor
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.as_ref().map(|s| (i, s)))
    {
        if !stack.item.starts_with("tconstruct:") {
            gold += u8::from(native_gold.is_some_and(|items| items.contains(stack.item.as_ref())));
            continue;
        }
        if stack.count != 1
            || stack.wire_patch().is_some()
            || !worn.contains(stack.item.as_ref())
            || !modifiable.contains(stack.item.as_ref())
        {
            return None;
        }
        // Source equipment-change and volatile flags remain active when broken.
        let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
        let mut levels = [0; 3];
        for (index, id) in ["chrysophilite", "gold_guard", "golden"]
            .into_iter()
            .enumerate()
        {
            let level = super::tconstruct_source_material_traits::level_with_upgrade(
                stack,
                &traits,
                &format!("tconstruct:{id}"),
            )?;
            if level < 0 {
                return None;
            }
            if level > 0 {
                let definition = super::tconstruct_modifier_snapshot::highest_definition(
                    raw,
                    &format!("tconstruct:tinkering/modifiers/{id}"),
                );
                let known = match index {
                    0 => super::tconstruct_gold_guard::known_chrysophilite(definition),
                    1 => super::tconstruct_gold_guard::known_guard(definition),
                    _ => super::tconstruct_gold_guard::known_golden(definition),
                };
                if !known {
                    return None;
                }
            }
            levels[index] = level;
        }
        active |= levels[0] > 0
            && super::tconstruct_fortune_harvest::armor_slot(&stack.item) == Some(slot);
        gold += u8::from(levels[0] >= 2 || levels[1] >= 2 || levels[2] > 0);
    }
    active.then(|| Context {
        gold,
        ore_items: ores.clone(),
    })
}

pub(super) fn apply(
    context: Option<&Context>,
    drops: &mut Vec<ItemStack>,
    random: &mut impl RandomSource,
) {
    let Some(context) = context else {
        return;
    };
    // Source post-condition examines actual generated loot. Silk Touch's ore
    // item suppresses the bonus even if it came from a custom loot table.
    if drops
        .iter()
        .any(|stack| stack.count > 0 && context.ore_items.contains(stack.item.as_ref()))
    {
        return;
    }
    if let Some(count) = rules::nuggets(context.gold, |bound| {
        random.next_int_bound(bound).unwrap_or(0)
    }) {
        drops.push(ItemStack::new("minecraft:gold_nugget", count));
    }
}
