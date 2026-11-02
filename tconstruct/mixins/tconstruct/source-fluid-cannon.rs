// Source fluid cannon's two-bucket tank and single whole-stack item handler.
use horizon_items::{item_nbt::{item_from_nbt, item_to_nbt}, items::ItemStack};
use horizon_world::nbt::Tag;
use super::tconstruct_source_melter_state::Tank;

#[derive(Clone, Debug)]
pub(super) struct Cannon { pub tank: Tank, pub item: Option<ItemStack> }
impl Cannon {
    pub(super) fn new() -> Self { Self { tank: Tank::new(2000), item: None } }
    pub(super) fn lossless(&self) -> bool {
        self.item.as_ref().is_none_or(|item| {
            if item.wire_patch().is_some() { return false; }
            item_from_nbt(&item_to_nbt(item)).is_some_and(|decoded| decoded.count == item.count && decoded.same_item_same_components(item))
        })
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
