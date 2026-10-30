// Ranged durability applies only to material-bearing TConstruct bows.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;

pub(super) fn is_material_bow(stack: &ItemStack) -> bool {
    matches!(&*stack.item, "tconstruct:longbow" | "tconstruct:crossbow")
        && matches!(stack.data().and_then(|data| data.other("minecraft:custom_data")).and_then(|tag| tag.get("tic_max_damage")), Some(Tag::Int(max)) if *max > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(id: &str, max: i32) -> ItemStack {
        let mut stack = ItemStack::new(id, 1);
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_max_damage", Tag::Int(max))])));
        stack
    }

    #[test]
    fn only_material_bows_use_retained_damage() {
        assert!(is_material_bow(&stack("tconstruct:longbow", 60)));
        assert!(is_material_bow(&stack("tconstruct:crossbow", 120)));
        assert!(!is_material_bow(&stack("minecraft:bow", 60)));
        assert!(!is_material_bow(&stack("tconstruct:pickaxe", 60)));
        assert!(!is_material_bow(&stack("tconstruct:longbow", 0)));
        assert!(!is_material_bow(&ItemStack::new("tconstruct:longbow", 1)));
    }
}
