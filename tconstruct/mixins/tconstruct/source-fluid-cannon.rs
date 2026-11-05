// Source fluid cannon's two-bucket tank and single whole-stack item handler.
use horizon_items::{item_nbt::{item_from_nbt, item_to_nbt}, items::ItemStack};
use horizon_world::nbt::Tag;
use super::tconstruct_source_melter_state::Tank;

#[derive(Clone, Debug)]
pub(super) struct Cannon { pub tank: Tank, pub item: Option<ItemStack> }
impl Cannon {
    pub(super) fn new() -> Self { Self { tank: Tank::new(2000), item: None } }
    pub(super) fn comparator(&self) -> i32 {
        if self.tank.amount == 0 { 0 } else { 1 + (14 * self.tank.amount / self.tank.capacity) as i32 }
    }
    pub(super) fn lossless(&self) -> bool {
        self.item.as_ref().is_none_or(|item| {
            if item.count == 0 || item.wire_patch().is_some() || item.data().is_some_and(|data| data.wire.patch().is_some()) { return false; }
            item_from_nbt(&item_to_nbt(item)).is_some_and(|decoded| decoded.count == item.count && decoded.damage == item.damage && decoded.same_item_same_components(item))
        })
    }
    /// A fluid-only operation must preserve the raw item and all unrelated
    /// machine/tank extensions. Never turn this into an inventory update.
    pub(super) fn tank_to_nbt_preserving(&self, raw: &Tag) -> Option<Tag> {
        let before = Self::from_nbt(raw)?;
        if Tank::from_nbt(&self.tank.to_nbt(), 2000).as_ref() != Some(&self.tank) { return None; }
        if !self.lossless() || self.item.as_ref().is_some_and(|item| item.data().is_some_and(|data| data.wire.patch().is_some())) { return None; }
        match (&before.item, &self.item) {
            (None, None) => {},
            (Some(old), Some(new)) if old.count == new.count && old.damage == new.damage && old.same_item_same_components(new) => {},
            _ => return None,
        }
        for alias in ["Tank", "Fluid"] { if raw.get(alias).is_some() { return None; } }
        let raw_tank = raw.get("tank")?;
        for alias in ["FluidName", "Amount", "Tag"] { if raw_tank.get(alias).is_some() { return None; } }
        if before.tank.to_nbt_preserving(raw_tank).as_ref() != Some(raw_tank) { return None; }
        let tank = self.tank.to_nbt_preserving(raw_tank)?;
        let Tag::Compound(mut fields) = raw.clone() else { return None; };
        fields.insert("tank".into(), tank);
        Some(Tag::Compound(fields))
    }
    /// Preserve native machine fields while changing a representable item.
    /// An opaque saved item can remain untouched, but cannot be removed,
    /// replaced or decremented through the lossy native item decoder.
    pub(super) fn to_nbt_preserving(&self, raw: &Tag) -> Option<Tag> {
        if !self.lossless() { return None; }
        let before = Self::from_nbt(raw)?;
        let mut tank_only = self.clone(); tank_only.item = before.item.clone();
        let Tag::Compound(mut fields) = tank_only.tank_to_nbt_preserving(raw)? else { return None; };
        let unchanged = match (&before.item, &self.item) {
            (None, None) => true,
            (Some(old), Some(new)) => old.count == new.count && old.damage == new.damage && old.same_item_same_components(new),
            _ => false,
        };
        if !unchanged {
            if raw.get("Items").is_some() { return None; }
            let old = before.item.as_ref().map(item_to_nbt).unwrap_or_else(|| Tag::compound(Vec::<(&str, Tag)>::new()));
            if raw.get("item") != Some(&old) { return None; }
            let item = self.item.as_ref().map(item_to_nbt).unwrap_or_else(|| Tag::compound(Vec::<(&str, Tag)>::new()));
            fields.insert("item".into(), item);
        }
        Some(Tag::Compound(fields))
    }
    pub(super) fn to_nbt(&self) -> Tag {
        let item = self.item.as_ref().map(item_to_nbt).unwrap_or_else(|| Tag::compound(Vec::<(&str, Tag)>::new()));
        Tag::compound([("tank", self.tank.to_nbt()), ("item", item)])
    }
    pub(super) fn from_nbt(value: &Tag) -> Option<Self> {
        let item = match value.get("item")? {
            Tag::Compound(fields) if fields.is_empty() => None,
            value => Some(item_from_nbt(value)?),
        };
        Some(Self { tank: Tank::from_nbt(value.get("tank")?, 2000)?, item })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn item_and_two_bucket_capacity_survive_save() {
        let mut cannon = Cannon::new();
        assert!(cannon.tank.insert("minecraft:lava", 2000));
        assert!(!cannon.tank.insert("minecraft:lava", 1));
        cannon.item = Some(ItemStack::new("minecraft:stone", 32));
        let restored = Cannon::from_nbt(&cannon.to_nbt()).unwrap();
        assert_eq!(restored.tank.amount, 2000);
        assert_eq!(restored.item.unwrap().count, 32);
    }
}
