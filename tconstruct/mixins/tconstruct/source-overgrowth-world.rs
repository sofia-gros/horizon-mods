use super::{tconstruct_source_overgrowth as rule, Game};
use horizon_world::nbt::Tag;

pub(super) fn tick(game: &mut Game<'_>) {
    if game.remote.is_some()
        || !game
            .player
            .as_ref()
            .is_some_and(|p| !p.health.dead_or_dying())
    {
        return;
    }
    let actor = game.native_acting_id();
    let phase = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_overgrowth_phase_mut()
        .entry(actor)
        .or_default();
    if !rule::second(phase) {
        return;
    }
    let Some(raw) = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()
        .and_then(|v| v.as_ref().ok())
    else {
        return;
    };
    if !rule::known(super::tconstruct_modifier_snapshot::highest_definition(
        raw,
        "tconstruct:tinkering/modifiers/overgrowth",
    )) {
        return;
    }
    let using = game.using.map(|(slot, _)| slot);
    let slots = (0..game.inventory.len() as i32)
        .chain((100..=103).rev())
        .chain(std::iter::once(-106));
    for slot in slots {
        if rule::active_slot(slot, using) {
            continue;
        }
        let Some(stack) = game.native_carried_stack(slot).cloned() else {
            continue;
        };
        if stack.count != 1 {
            continue;
        }
        let Some(tags) = game
            .world
            .chunks
            .__horizon_mixins_tconstruct_resolved_item_tags()
            .as_ref()
            .and_then(|v| v.as_ref().ok())
        else {
            continue;
        };
        if !tags
            .get("tconstruct:modifiable")
            .is_some_and(|tag| tag.contains(stack.item.as_ref()))
        {
            continue;
        }
        let Some(traits) = super::tconstruct_source_material_traits::levels(&game.world, &stack)
        else {
            continue;
        };
        let Some(level) =
            super::tconstruct_source_material_traits::level_with_upgrade(&stack, &traits, rule::ID)
        else {
            continue;
        };
        let Some(overworked) = super::tconstruct_source_material_traits::level_with_upgrade(
            &stack,
            &traits,
            "tconstruct:overworked",
        ) else {
            continue;
        };
        let Some(mut custom) = stack
            .data()
            .and_then(|d| d.other("minecraft:custom_data"))
            .cloned()
        else {
            continue;
        };
        let Tag::Compound(ref mut fields) = custom else {
            continue;
        };
        if !matches!(
            fields.get(&"tic_overslime_active".into()),
            Some(Tag::Byte(1))
        ) {
            continue;
        }
        let (Some(Tag::Int(current)), Some(Tag::Int(capacity))) = (
            fields.get(&"tic_overslime".into()),
            fields.get(&"tic_overslime_capacity".into()),
        ) else {
            continue;
        };
        if !rule::eligible(*current, *capacity, level) {
            continue;
        }
        let Some(next) = rule::recover(
            *current,
            *capacity,
            level,
            overworked,
            game.world.random.next_float(),
        ) else {
            continue;
        };
        fields.insert("tic_overslime".into(), Tag::Int(next));
        // No callbacks occur between snapshot and write; change only this field.
        if let Some(live) = game.native_carried_stack(slot) {
            live.data_mut()
                .set_other("minecraft:custom_data", Some(custom));
        }
    }
}
