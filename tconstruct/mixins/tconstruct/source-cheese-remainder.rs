use horizon_items::items::ItemStack;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Remainder { Creative, ReplaceHand, Inventory }
/// Commit only if the original held stack is still intact at finish.
pub(super) fn plan(before: &ItemStack, held: &Option<ItemStack>, creative: bool) -> Option<Remainder> {
    if &*before.item != "tconstruct:cheese_block" || before.count == 0 || before.damage != 0 || before.extra.is_some() || before.wire_patch().is_some() { return None; }
    let held = held.as_ref()?;
    if held.count != before.count || held.damage != before.damage || !held.same_item_same_components(before) { return None; }
    if creative { Some(Remainder::Creative) } else if before.count == 1 { Some(Remainder::ReplaceHand) } else { Some(Remainder::Inventory) }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plans_creative_last_and_stacked_without_mutating() {
        let one=ItemStack::new("tconstruct:cheese_block",1); let two=ItemStack::new("tconstruct:cheese_block",2);
        assert_eq!(plan(&one,&Some(one.clone()),true),Some(Remainder::Creative));
        assert_eq!(plan(&one,&Some(one.clone()),false),Some(Remainder::ReplaceHand));
        assert_eq!(plan(&two,&Some(two.clone()),false),Some(Remainder::Inventory));
        assert_eq!(plan(&two,&Some(one),false),None);
        assert_eq!(plan(&two,&None,false),None);
        assert_eq!(plan(&ItemStack::new("minecraft:stone",1),&None,false),None);
    }
    #[test]
    fn refuses_changed_components_or_damage() {
        let before=ItemStack::new("tconstruct:cheese_block",2); let mut held=before.clone(); held.damage=1;
        assert_eq!(plan(&before,&Some(held),false),None);
        let mut custom=before.clone();
        custom.data_mut().set_other("minecraft:custom_data", Some(horizon_world::nbt::Tag::compound([("example",horizon_world::nbt::Tag::Int(1))])));
        assert_eq!(plan(&custom,&Some(custom.clone()),false),None);
    }
}
