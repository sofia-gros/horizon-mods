// Source Casting Tank: a 4-bucket tank and one input/output container pair.
use horizon_items::{item_nbt::{item_from_nbt, item_to_nbt}, items::ItemStack};
use horizon_world::nbt::Tag;
use super::tconstruct_source_melter_state::Tank;

#[derive(Clone, Debug)]
pub(super) struct CastingTank {
    pub tank: Tank,
    pub slots: [Option<ItemStack>; 2],
    pub redstone: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn comparator_uses_source_fluid_fraction() {
        let mut state = CastingTank::new();
        assert_eq!(state.comparator(), 0);
        assert!(state.tank.insert("minecraft:water", 1));
        assert_eq!(state.comparator(), 1);
        assert!(state.tank.insert("minecraft:water", 3999));
        assert_eq!(state.comparator(), 15);
    }
    #[test]
    fn one_bucket_moves_only_after_output_is_free_and_survives_save() {
        let mut state = CastingTank::new();
        state.slots[0] = Some(ItemStack::new("minecraft:lava_bucket", 1));
        state.slots[1] = Some(ItemStack::new("minecraft:stone", 1));
        let before = state.to_nbt();
        assert!(!state.process(&|_| true, &|_| true, None, false));
        assert_eq!(state.to_nbt(), before);
        state.slots[1] = None;
        assert!(state.process(&|_| true, &|_| true, None, false));
        assert_eq!(state.tank.amount, 1000);
        assert!(state.slots[0].is_none());
        assert_eq!(state.slots[1].as_ref().unwrap().item.as_ref(), "minecraft:bucket");
        assert_eq!(CastingTank::from_nbt(&state.to_nbt()).unwrap().to_nbt(), state.to_nbt());
    }
}
impl CastingTank {
    pub(super) fn new() -> Self { Self { tank: Tank::new(4000), slots: [None, None], redstone: false } }
    pub(super) fn comparator(&self) -> i32 {
        if self.tank.amount == 0 { 0 } else { 1 + (14 * self.tank.amount / self.tank.capacity) as i32 }
    }
    pub(super) fn lossless(&self) -> bool {
        self.slots.iter().flatten().all(|stack| horizon_client::pluto_stack_storage::encode_item_lossless(stack).is_some())
    }
    /// Preserve native extensions and unchanged raw slots. Changing an opaque
    /// saved item must wait rather than discard data omitted by its decoder.
    pub(super) fn to_nbt_preserving(&self, raw: &Tag) -> Option<Tag> {
        if !self.lossless() || Tank::from_nbt(&self.tank.to_nbt(), 4000).as_ref() != Some(&self.tank) { return None; }
        let before = Self::from_nbt(raw)?;
        for alias in ["Items", "Tank", "Fluid", "lastRedstone"] { if raw.get(alias).is_some() { return None; } }
        let raw_tank = raw.get("tank")?;
        for alias in ["FluidName", "Amount", "Tag"] { if raw_tank.get(alias).is_some() { return None; } }
        if before.tank.to_nbt_preserving(raw_tank).as_ref() != Some(raw_tank) { return None; }
        let tank = self.tank.to_nbt_preserving(raw_tank)?;
        let Tag::List(mut slots) = raw.get("slots")?.clone() else { return None; };
        for index in 0..2 {
            let unchanged = match (&before.slots[index], &self.slots[index]) {
                (None, None) => true,
                (Some(old), Some(new)) => old.count == new.count && old.damage == new.damage && old.same_item_same_components(new),
                _ => false,
            };
            if unchanged { continue; }
            let old = before.slots[index].as_ref().map(item_to_nbt).unwrap_or_else(|| Tag::compound(Vec::<(&str, Tag)>::new()));
            if slots[index] != old { return None; }
            slots[index] = self.slots[index].as_ref().map(item_to_nbt).unwrap_or_else(|| Tag::compound(Vec::<(&str, Tag)>::new()));
        }
        let Tag::Compound(mut fields) = raw.clone() else { return None; };
        fields.insert("tank".into(), tank); fields.insert("slots".into(), Tag::List(slots));
        fields.insert("redstone".into(), Tag::Byte(self.redstone as i8));
        Some(Tag::Compound(fields))
    }
    pub(super) fn to_nbt(&self) -> Tag {
        let slots = self.slots.iter().map(|slot| slot.as_ref().map(item_to_nbt)
            .unwrap_or_else(|| Tag::compound(Vec::<(&str,Tag)>::new()))).collect();
        Tag::compound([("tank", self.tank.to_nbt()), ("slots", Tag::List(slots)), ("redstone", Tag::Byte(self.redstone as i8))])
    }
    pub(super) fn from_nbt(value: &Tag) -> Option<Self> {
        let Tag::List(slots) = value.get("slots")? else { return None; };
        if slots.len() != 2 { return None; }
        let read = |value: &Tag| -> Option<Option<ItemStack>> {
            if matches!(value, Tag::Compound(fields) if fields.is_empty()) { Some(None) }
            else { Some(Some(item_from_nbt(value)?)) }
        };
        let redstone = match value.get("redstone")? { Tag::Byte(0) => false, Tag::Byte(1) => true, _ => return None };
        Some(Self { tank: Tank::from_nbt(value.get("tank")?, 4000)?, slots: [read(&slots[0])?, read(&slots[1])?], redstone })
    }
    /// Both the machine and slots are staged by the existing container
    /// transfer; failure leaves the input and tank untouched.
    pub(super) fn process(&mut self, known_item: &impl Fn(&str) -> bool, known_fluid: &impl Fn(&str) -> bool,
        tags: Option<&std::collections::HashMap<String, std::collections::HashSet<String>>>, allow_color: bool) -> bool {
        if !self.lossless() || self.slots[0].as_ref().is_none_or(|stack| stack.count != 1) || self.slots[1].is_some() { return false; }
        super::tconstruct_source_fluid_container::transfer_single_with_color(&mut self.tank, &mut self.slots,
            super::tconstruct_source_fluid_container::Direction::Auto, known_item, known_fluid, tags, allow_color)
    }
}
