// Remove only the supported tip value on a copy. The caller validates the
// source recipe, item/fluid tags, and effective modifier level before using it.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;

pub(super) fn clear(stack: &ItemStack, modifier_level: Option<i32>) -> Option<ItemStack> {
    if !modifier_level.is_some_and(|level| level > 0) || stack.count != 1 || stack.wire_patch().is_some() { return None; }
    let mut custom = stack.data()?.other("minecraft:custom_data")?.clone();
    let Tag::Compound(ref mut fields) = custom else { return None; };
    let Tag::Compound(persistent) = fields.get_mut(&"tic_persistent".into())? else { return None; };
    match persistent.get(&"tconstruct:tipped".into()) {
        Some(Tag::String(value)) if !value.to_string_lossy().is_empty() => {},
        _ => return None,
    }
    persistent.remove(&"tconstruct:tipped".into());
    let mut output = stack.clone();
    output.data_mut().set_other("minecraft:custom_data", Some(custom));
    // Casting persists both the input and the reserved result. Refuse opaque
    // components that the host item NBT codec cannot round-trip.
    let lossless = |item: &ItemStack| {
        horizon_items::item_nbt::item_from_nbt(&horizon_items::item_nbt::item_to_nbt(item))
            .is_some_and(|decoded| decoded.count == item.count && decoded.damage == item.damage
                && decoded.same_item_same_components(item))
    };
    (lossless(stack) && lossless(&output)).then_some(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clear_preserves_item_and_unrelated_data() {
        let mut stack = ItemStack::new("tconstruct:arrow", 1);
        stack.damage = 3;
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
            ("other", Tag::Int(7)),
            ("tic_persistent", Tag::compound([
                ("tconstruct:tipped", Tag::String("minecraft:poison".into())),
                ("other", Tag::Int(9)),
            ])),
        ])));
        assert!(clear(&stack, None).is_none());
        assert!(clear(&stack, Some(0)).is_none());
        assert!(clear(&stack, Some(-1)).is_none());
        let output = clear(&stack, Some(1)).unwrap();
        let custom = output.data().unwrap().other("minecraft:custom_data").unwrap();
        assert_eq!(output.damage, 3);
        assert_eq!(custom.get("other"), Some(&Tag::Int(7)));
        assert_eq!(custom.get("tic_persistent").unwrap().get("other"), Some(&Tag::Int(9)));
        assert_eq!(custom.get("tic_persistent").unwrap().get("tconstruct:tipped"), None);
        assert!(stack.data().unwrap().other("minecraft:custom_data").unwrap().get("tic_persistent").unwrap().get("tconstruct:tipped").is_some());
    }
    #[test]
    fn reserved_cast_survives_reload_and_requires_complete_water_and_revalidation() {
        let mut input = ItemStack::new("tconstruct:arrow", 1);
        input.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_persistent", Tag::compound([
            ("tconstruct:tipped", Tag::String("minecraft:poison".into())), ("unrelated", Tag::Int(9))
        ]))])));
        let output = clear(&input, Some(1)).unwrap();
        let mut state = super::super::tconstruct_source_melter_state::Casting::new();
        state.input = Some(input);
        assert!(state.prepare("test:clear", 50, 20, output, true, false));
        assert!(state.tank.insert("minecraft:water", 49));
        assert!(!state.tick(true));
        assert!(state.tank.insert("minecraft:water", 1));
        assert!(!state.tick(false)); assert_eq!(state.timer, 0);
        let mut restored = super::super::tconstruct_source_melter_state::Casting::from_nbt(&state.to_nbt()).unwrap();
        for _ in 0..19 { assert!(!restored.tick(true)); }
        assert!(restored.tick(true)); assert!(restored.input.is_none());
        assert_eq!(restored.tank.amount, 0);
        let custom = restored.output.as_ref().unwrap().data().unwrap().other("minecraft:custom_data").unwrap();
        let persistent = custom.get("tic_persistent").unwrap();
        assert_eq!(persistent.get("tconstruct:tipped"), None);
        assert_eq!(persistent.get("unrelated"), Some(&Tag::Int(9)));
    }
    #[test]
    fn rejects_missing_empty_and_malformed_tip_and_stacks() {
        let mut stack = ItemStack::new("tconstruct:arrow", 1);
        assert!(clear(&stack, Some(1)).is_none());
        for tip in [Tag::Int(1), Tag::String("".into())] {
            stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_persistent", Tag::compound([("tconstruct:tipped", tip)]))])));
            assert!(clear(&stack, Some(1)).is_none());
        }
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_persistent", Tag::compound([("tconstruct:tipped", Tag::String("minecraft:poison".into()))]))])));
        stack.count = 2;
        assert!(clear(&stack, Some(1)).is_none());
    }
}
