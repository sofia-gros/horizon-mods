// A real one-item filter. Unknown/legacy state is never converted implicitly.
use horizon_items::{item_nbt::{item_from_nbt, item_to_nbt}, items::ItemStack};
use horizon_world::nbt::Tag;

pub(super) fn is_duct(block: &str) -> bool {
    matches!(block, "tconstruct:seared_duct" | "tconstruct:scorched_duct")
}

#[derive(Clone, Debug, Default)]
pub(super) struct Duct { pub item: Option<ItemStack> }
impl Duct {
    pub(super) fn to_nbt(&self) -> Tag {
        Tag::compound([("item", self.item.as_ref().map(item_to_nbt)
            .unwrap_or_else(|| Tag::compound(Vec::<(&str, Tag)>::new())))])
    }
    pub(super) fn from_nbt(raw: &Tag) -> Option<Self> {
        let item = match raw.get("item")? {
            Tag::Compound(fields) if fields.is_empty() => None,
            raw => {
                let item = item_from_nbt(raw)?;
                if item.count != 1 || item.wire_patch().is_some() || item_to_nbt(&item) != *raw { return None; }
                Some(item)
            },
        };
        let state = Self { item };
        (state.to_nbt() == *raw).then_some(state)
    }
    pub(super) fn from_envelope(raw: &Tag, pos: (i32, i32, i32)) -> Option<Self> {
        let Tag::Compound(fields) = raw else { return None; };
        if fields.keys().any(|key| !matches!(key.to_string().ok().as_deref(),
            Some("id" | "x" | "y" | "z" | "Items" | "tconstruct:duct"))) { return None; }
        if raw.get("id") != Some(&Tag::String("tconstruct:duct".into())) { return None; }
        for (key, value) in [("x", pos.0), ("y", pos.1), ("z", pos.2)] {
            if raw.get(key) != Some(&Tag::Int(value)) { return None; }
        }
        // The live chest is a zero-slot save projection, never another inventory.
        if raw.get("Items").is_some_and(|items| !matches!(items, Tag::List(items) if items.is_empty())) { return None; }
        Self::from_nbt(raw.get("tconstruct:duct")?)
    }
    pub(super) fn accepts_stack(stack: &ItemStack) -> bool {
        if stack.count == 0 || stack.wire_patch().is_some() { return false; }
        let mut one = stack.clone(); one.count = 1;
        item_from_nbt(&item_to_nbt(&one)).is_some_and(|decoded|
            decoded.count == 1 && decoded.same_item_same_components(&one))
    }
}

/// A filter selects the first exact fluid+NBT layer, not the controller's
/// first layer. Its container volume never limits the drained amount.
pub(super) fn selected<'a>(fluids: &'a [super::tconstruct_source_smeltery_tank::FluidAmount],
    filter: Option<&(String, Option<Tag>)>) -> Option<&'a super::tconstruct_source_smeltery_tank::FluidAmount> {
    let (id, tag) = filter?;
    fluids.iter().find(|fluid| fluid.amount > 0 && fluid.id == *id && fluid.tag == *tag)
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn duct_filter_roundtrip_keeps_one_item_and_rejects_unknown_state() {
        let duct = Duct { item: Some(ItemStack::new("minecraft:water_bucket", 1)) };
        assert_eq!(Duct::from_nbt(&duct.to_nbt()).unwrap().to_nbt(), duct.to_nbt());
        let mut opaque = duct.to_nbt();
        if let Tag::Compound(fields) = &mut opaque { fields.insert("future".into(), Tag::Int(1)); }
        assert!(Duct::from_nbt(&opaque).is_none());
        assert!(Duct::from_nbt(&Duct { item: Some(ItemStack::new("minecraft:water_bucket", 2)) }.to_nbt()).is_none());
    }
    #[test] fn duct_empty_filter_never_selects_and_tag_identity_selects_later_layer() {
        use super::super::tconstruct_source_smeltery_tank::FluidAmount;
        let a = Tag::compound([("Potion", Tag::String("minecraft:healing".into()))]);
        let b = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into()))]);
        let fluids = vec![FluidAmount { id: "tconstruct:potion".into(), tag: Some(a), amount: 90 },
            FluidAmount { id: "tconstruct:potion".into(), tag: Some(b.clone()), amount: 180 }];
        assert!(selected(&fluids, None).is_none());
        assert!(selected(&fluids, Some(&("tconstruct:potion".into(), None))).is_none());
        assert_eq!(selected(&fluids, Some(&("tconstruct:potion".into(), Some(b)))).unwrap().amount, 180);
    }
    #[test] fn duct_envelope_refuses_legacy_opaque_duplicate_inventory_and_bad_coordinates() {
        let native = Tag::compound([("id", Tag::String("tconstruct:duct".into())),
            ("x", Tag::Int(1)), ("y", Tag::Int(2)), ("z", Tag::Int(3)),
            ("Items", Tag::List(vec![])), ("tconstruct:duct", Duct::default().to_nbt())]);
        assert!(Duct::from_envelope(&native, (1,2,3)).unwrap().item.is_none());
        assert!(Duct::from_envelope(&native, (1,2,4)).is_none());
        for (key, value) in [("item", item_to_nbt(&ItemStack::new("minecraft:water_bucket", 1))),
            ("future", Tag::Int(1)), ("Items", Tag::List(vec![Tag::compound([("Slot",Tag::Byte(0))])]))] {
            let mut raw = native.clone();
            if let Tag::Compound(fields) = &mut raw { fields.insert(key.into(), value); }
            assert!(Duct::from_envelope(&raw, (1,2,3)).is_none());
        }
    }
}
