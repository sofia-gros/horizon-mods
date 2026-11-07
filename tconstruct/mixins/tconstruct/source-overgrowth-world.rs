// PR706 policy on the native carried-stack callback; no actor/phase side table.
use super::{tconstruct_source_overgrowth as rule, Game};
use horizon_world::nbt::Tag;

pub(super) fn tick(game: &mut Game<'_>, slot: i32) {
    if !game.native_on("tconstruct") || game.remote.is_some() {
        return;
    }
    let Some(player) = game.player.as_ref() else { return; };
    if player.health.dead_or_dying() || !rule::second(player.health.tick_count)
        || rule::active_slot(slot, game.using.map(|(slot, _)| slot))
    {
        return;
    }
    let Some(stack) = game.native_carried_stack(slot).cloned() else { return; };
    if stack.count != 1 { return; }
    let Some(tags) = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags()
        .as_ref().and_then(|v| v.as_ref().ok()) else { return; };
    if !tags.get("tconstruct:modifiable")
        .is_some_and(|tag| tag.contains(stack.item.as_ref())) { return; }
    let Some(raw) = game.world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref().and_then(|v| v.as_ref().ok()) else { return; };
    if !rule::known(super::tconstruct_modifier_snapshot::highest_definition(
        raw, "tconstruct:tinkering/modifiers/overgrowth",
    )) { return; }
    let Some(traits) = super::tconstruct_source_material_traits::levels(&game.world, &stack)
        else { return; };
    let Some(level) = super::tconstruct_source_material_traits::level_with_upgrade(
        &stack, &traits, rule::ID,
    ) else { return; };
    let Some(overworked) = super::tconstruct_source_material_traits::level_with_upgrade(
        &stack, &traits, "tconstruct:overworked",
    ) else { return; };
    let Some(custom) = stack.data().and_then(|d| d.other("minecraft:custom_data"))
        else { return; };
    let Some(next) = prepare(custom, level, overworked,
        super::tconstruct_modifier_random::next_float,
    ) else { return; };
    // Preparation has no callbacks or live writes. This invocation owns the slot.
    if let Some(live) = game.native_carried_stack(slot) {
        live.data_mut().set_other("minecraft:custom_data", Some(next));
    }
}

fn prepare(custom: &Tag, level: i32, overworked: i32, draw: impl FnOnce() -> f32) -> Option<Tag> {
    let Tag::Compound(fields) = custom else { return None; };
    if !matches!(fields.get(&"tic_overslime_active".into()), Some(Tag::Byte(1))) {
        return None;
    }
    let current = match fields.get(&"tic_overslime".into()) {
        Some(Tag::Int(value)) => *value,
        None => 0, // Native TC capacity bars omit an empty amount.
        _ => return None,
    };
    let Some(Tag::Int(capacity)) = fields.get(&"tic_overslime_capacity".into())
        else { return None; };
    if !rule::eligible(current, *capacity, level) || overworked < 0 { return None; }
    let next = rule::recover(current, *capacity, level, overworked, draw())?;
    horizon_client::pluto_nbt_storage::patch_compound(
        custom, [("tic_overslime", Some(Tag::Int(next)))],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_world::nbt::{Compound, JavaString};

    fn custom(amount: Option<Tag>) -> Tag {
        let mut fields = Compound::new();
        fields.insert("tic_overslime_active".into(), Tag::Byte(1));
        fields.insert("tic_overslime_capacity".into(), Tag::Int(50));
        if let Some(amount) = amount { fields.insert("tic_overslime".into(), amount); }
        Tag::Compound(fields)
    }

    #[test]
    fn shared_patch_changes_only_amount_and_keeps_opaque_data() {
        let Tag::Compound(mut fields) = custom(Some(Tag::Int(3))) else { unreachable!() };
        let key = JavaString(vec![0xd800]);
        let value = Tag::List(vec![Tag::String(JavaString(vec![0xdc00])), Tag::Float(0x7fc01234)]);
        fields.insert(key.clone(), value.clone());
        let original = Tag::Compound(fields);
        let next = prepare(&original, 1, 2, || 0.0).unwrap();
        assert_eq!(next.get("tic_overslime"), Some(&Tag::Int(6)));
        let Tag::Compound(fields) = next else { unreachable!() };
        assert_eq!(fields.get(&key), Some(&value));
        assert_eq!(original.get("tic_overslime"), Some(&Tag::Int(3)));
    }

    #[test]
    fn absent_empty_amount_recovers_but_wrong_typed_amount_does_not() {
        assert_eq!(prepare(&custom(None), 1, 0, || 0.0).unwrap().get("tic_overslime"), Some(&Tag::Int(1)));
        assert!(prepare(&custom(Some(Tag::String("0".into()))), 1, 0, || panic!("no draw")).is_none());
    }

    #[test]
    fn invalid_full_or_inactive_stack_does_not_draw_or_patch() {
        for source in [Tag::Int(1), custom(Some(Tag::Int(50))), custom(Some(Tag::Int(-1))),
            Tag::compound([("tic_overslime_active", Tag::Byte(0))])]
        {
            assert!(prepare(&source, 1, 0, || panic!("no draw")).is_none());
        }
        assert!(prepare(&custom(Some(Tag::Int(3))), 0, 0, || panic!("no draw")).is_none());
        assert!(prepare(&custom(Some(Tag::Int(3))), 1, -1, || panic!("no draw")).is_none());
    }

    #[test]
    fn eligible_stack_draws_once_and_failed_chance_keeps_original() {
        let calls = std::cell::Cell::new(0);
        let source = custom(Some(Tag::Int(3)));
        let next = prepare(&source, 1, 0, || { calls.set(calls.get() + 1); 0.05 });
        assert!(next.is_none());
        assert_eq!(calls.get(), 1);
        assert_eq!(source.get("tic_overslime"), Some(&Tag::Int(3)));
    }
}
