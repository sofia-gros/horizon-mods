// Multi-fluid storage and atomic alloy operations for a Smeltery.
//
// Recipe tags and conditions are resolved by the caller against loaded data.
// This module does not select recipes or alter a world by itself.
use horizon_world::nbt::{Compound, Tag};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FluidAmount {
    pub id: String,
    pub tag: Option<Tag>,
    pub amount: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SmelteryTank {
    pub capacity: u32,
    pub fluids: Vec<FluidAmount>,
}

impl SmelteryTank {
    pub(super) fn new(capacity: u32) -> Self { Self { capacity, fluids: Vec::new() } }

    pub(super) fn total(&self) -> u32 {
        self.fluids.iter().fold(0u32, |n, fluid| n.saturating_add(fluid.amount))
    }

    pub(super) fn amount(&self, id: &str) -> u32 {
        self.amount_tagged(id, None)
    }
    pub(super) fn amount_tagged(&self, id: &str, tag: Option<&Tag>) -> u32 {
        self.fluids.iter().find(|f| f.id == id && f.tag.as_ref() == tag).map_or(0, |f| f.amount)
    }

    pub(super) fn insert(&mut self, id: &str, amount: u32) -> bool {
        self.insert_tagged(id, None, amount)
    }
    pub(super) fn insert_tagged(&mut self, id: &str, tag: Option<&Tag>, amount: u32) -> bool {
        if tag.is_some_and(|tag| !matches!(tag, Tag::Compound(_))) || id.is_empty() || amount == 0 || amount > self.capacity.saturating_sub(self.total()) { return false; }
        if let Some(fluid) = self.fluids.iter_mut().find(|f| f.id == id && f.tag.as_ref() == tag) {
            fluid.amount += amount;
        } else {
            self.fluids.push(FluidAmount { id: id.to_owned(), tag: tag.cloned(), amount });
        }
        true
    }

    pub(super) fn take(&mut self, id: &str, amount: u32) -> bool {
        self.take_tagged(id, None, amount)
    }
    pub(super) fn take_tagged(&mut self, id: &str, tag: Option<&Tag>, amount: u32) -> bool {
        if amount == 0 { return false; }
        let Some(index) = self.fluids.iter().position(|f| f.id == id && f.tag.as_ref() == tag && f.amount >= amount) else { return false; };
        self.fluids[index].amount -= amount;
        if self.fluids[index].amount == 0 { self.fluids.remove(index); }
        true
    }

    pub(super) fn move_to_bottom(&mut self, index: usize) -> bool {
        if index >= self.fluids.len() { return false; }
        if index != 0 { let fluid = self.fluids.remove(index); self.fluids.insert(0, fluid); }
        true
    }

    /// The source Foundry requires space for the primary result. Once that
    /// succeeds it fills each byproduct in order, accepting partial amounts
    /// when the remaining tank capacity runs out. Any remainder is discarded,
    /// including when the primary is zero and the tank is already full. This
    /// matches upstream f4e77d43: tryFillTank checks only the primary, then
    /// handleByproducts ignores the amount accepted by each fill.
    pub(super) fn foundry_output(&mut self, primary: &FluidAmount, byproducts: &[FluidAmount]) -> bool {
        if primary.tag.is_some() || byproducts.iter().any(|fluid| fluid.tag.is_some()) { return false; }
        let mut next = self.clone();
        if primary.id.is_empty() || (primary.amount != 0 && !next.insert(&primary.id, primary.amount)) { return false; }
        for fluid in byproducts {
            let amount = fluid.amount.min(next.capacity.saturating_sub(next.total()));
            if amount > 0 { let _ = next.insert(&fluid.id, amount); }
        }
        *self = next;
        true
    }

    /// Commit only when every input and the resulting net volume fit.
    pub(super) fn alloy(&mut self, inputs: &[FluidAmount], result: &FluidAmount) -> bool {
        if inputs.iter().any(|fluid| fluid.tag.is_some()) || result.tag.is_some() || result.id.is_empty() || result.amount == 0 { return false; }
        let mut next = self.clone();
        let mut removed = 0u32;
        for input in inputs {
            if !next.take(&input.id, input.amount) { return false; }
            let Some(sum) = removed.checked_add(input.amount) else { return false; };
            removed = sum;
        }
        // The source allows a net decrease even if a resized tank is overfull.
        if result.amount.saturating_sub(removed) > self.capacity.saturating_sub(self.total()) { return false; }
        if let Some(fluid) = next.fluids.iter_mut().find(|f| f.id == result.id && f.tag.is_none()) {
            let Some(amount) = fluid.amount.checked_add(result.amount) else { return false; };
            fluid.amount = amount;
        } else { next.fluids.push(result.clone()); }
        if next.fluids.iter().try_fold(0u32, |sum, fluid| sum.checked_add(fluid.amount)).is_none() { return false; }
        *self = next;
        true
    }

    pub(super) fn to_nbt(&self) -> Tag {
        let fluids = self.fluids.iter().map(|fluid| {
            let mut fields = Compound::new();
            fields.insert("id".into(), Tag::String(fluid.id.as_str().into()));
            fields.insert("amount".into(), Tag::Long(i64::from(fluid.amount)));
            if let Some(tag) = &fluid.tag { fields.insert("fluid_tag".into(), tag.clone()); }
            Tag::Compound(fields)
        }).collect();
        Tag::compound([("capacity", Tag::Long(i64::from(self.capacity))), ("fluids", Tag::List(fluids))])
    }

    /// Keep native tank and row extensions attached to their exact fluid/tag
    /// identity. Removing an opaque row would discard unknown state, so wait.
    pub(super) fn to_nbt_preserving(&self, raw: &Tag) -> Option<Tag> {
        let before = Self::from_nbt(raw)?;
        let canonical = self.to_nbt();
        if Self::from_nbt(&canonical).as_ref() != Some(self) { return None; }
        let Tag::Compound(_) = raw else { return None; };
        for alias in ["Capacity", "Fluids", "Fluid", "Tank", "FluidName", "Amount", "Tag"] {
            if raw.get(alias).is_some() { return None; }
        }
        let Tag::List(old_rows) = raw.get("fluids")? else { return None; };
        let old_canonical = before.to_nbt();
        let Tag::List(old_normal) = old_canonical.get("fluids")? else { return None; };
        let Tag::List(new_normal) = canonical.get("fluids")? else { return None; };
        for (index, fluid) in before.fluids.iter().enumerate() {
            let row = &old_rows[index];
            let Tag::String(id) = row.get("id")? else { return None; };
            if id.to_string().ok()?.as_str() != fluid.id.as_str() { return None; }
            for alias in ["FluidName", "Amount", "Tag", "fluid"] { if row.get(alias).is_some() { return None; } }
            if !self.fluids.iter().any(|next| next.id == fluid.id && next.tag == fluid.tag)
                && row != &old_normal[index] { return None; }
        }
        let mut rows = Vec::with_capacity(self.fluids.len());
        for (index, fluid) in self.fluids.iter().enumerate() {
            if let Some(old) = before.fluids.iter().position(|old| old.id == fluid.id && old.tag == fluid.tag) {
                rows.push(horizon_client::pluto_nbt_storage::patch_compound(
                    &old_rows[old], [("amount", Some(Tag::Long(i64::from(fluid.amount))))])?);
            } else { rows.push(new_normal[index].clone()); }
        }
        horizon_client::pluto_nbt_storage::patch_compound(raw, [
            ("capacity", Some(Tag::Long(i64::from(self.capacity)))),
            ("fluids", Some(Tag::List(rows))),
        ])
    }

    pub(super) fn from_nbt(tag: &Tag) -> Option<Self> {
        let Tag::Long(capacity) = tag.get("capacity")? else { return None; };
        let mut tank = Self::new(u32::try_from(*capacity).ok()?);
        let Tag::List(fluids) = tag.get("fluids")? else { return None; };
        let mut total = 0u32;
        for fluid in fluids {
            let Tag::String(id) = fluid.get("id")? else { return None; };
            let Tag::Long(amount) = fluid.get("amount")? else { return None; };
            let id = id.to_string_lossy();
            let amount = u32::try_from(*amount).ok()?;
            let metadata = match fluid.get("fluid_tag") { None => None, Some(value @ Tag::Compound(_)) => Some(value.clone()), _ => return None };
            if id.is_empty() || amount == 0 || tank.amount_tagged(&id, metadata.as_ref()) != 0 { return None; }
            total = total.checked_add(amount)?;
            tank.fluids.push(FluidAmount { id, tag: metadata, amount });
        }
        Some(tank)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fluid(id: &str, amount: u32) -> FluidAmount { FluidAmount { id: id.into(), tag: None, amount } }
    #[test]
    fn foundry_requires_primary_space_and_fills_byproducts_in_order() {
        let mut tank = SmelteryTank::new(205);
        let primary = fluid("copper", 180);
        let byproducts = [fluid("gold", 30), fluid("silver", 10)];
        assert!(tank.foundry_output(&primary, &byproducts));
        assert_eq!(tank.fluids, vec![fluid("copper", 180), fluid("gold", 25)]);
        let before = tank.clone();
        assert!(!tank.foundry_output(&primary, &byproducts));
        assert_eq!(tank, before);
    }
    #[test]
    fn combines_source_bronze_volumes_in_a_full_tank() {
        let mut tank = SmelteryTank::new(360);
        assert!(tank.insert("copper", 270)); assert!(tank.insert("tin", 90));
        assert!(tank.alloy(&[fluid("copper", 270), fluid("tin", 90)], &fluid("bronze", 360)));
        assert_eq!(tank.fluids, vec![fluid("bronze", 360)]);
    }
    #[test]
    fn missing_input_or_capacity_leaves_all_fluids_intact() {
        let mut tank = SmelteryTank::new(360);
        assert!(tank.insert("copper", 270)); assert!(tank.insert("tin", 90));
        let before = tank.clone();
        assert!(!tank.alloy(&[fluid("copper", 270), fluid("tin", 91)], &fluid("bronze", 360)));
        assert_eq!(tank, before);
        assert!(!tank.alloy(&[fluid("copper", 270), fluid("tin", 90)], &fluid("bronze", 361)));
        assert_eq!(tank, before);
    }
    #[test]
    fn duplicate_inputs_cannot_overdraw() {
        let mut tank = SmelteryTank::new(100); assert!(tank.insert("copper", 90));
        let before = tank.clone();
        assert!(!tank.alloy(&[fluid("copper", 60), fluid("copper", 60)], &fluid("result", 90)));
        assert_eq!(tank, before);
    }
    #[test]
    fn multiple_fluids_survive_nbt_round_trip() {
        let mut tank = SmelteryTank::new(2160);
        assert!(tank.insert("tconstruct:molten_copper", 270));
        assert!(tank.insert("tconstruct:molten_tin", 90));
        assert_eq!(SmelteryTank::from_nbt(&tank.to_nbt()), Some(tank));
    }
    #[test]
    fn structure_shrink_preserves_existing_fluid_until_drained() {
        let mut tank = SmelteryTank::new(2160);
        assert!(tank.insert("tconstruct:molten_copper", 1800));
        tank.capacity = 1080;
        let mut restored = SmelteryTank::from_nbt(&tank.to_nbt()).unwrap();
        assert_eq!(restored.total(), 1800);
        assert!(!restored.insert("tconstruct:molten_tin", 1));
        assert!(restored.take("tconstruct:molten_copper", 800));
        assert!(restored.insert("tconstruct:molten_tin", 80));
    }
    #[test]
    fn overfull_tank_can_alloy_when_volume_does_not_increase() {
        let mut tank = SmelteryTank::new(500);
        assert!(tank.insert("copper", 270)); assert!(tank.insert("amethyst", 100));
        tank.capacity = 100;
        assert!(tank.alloy(&[fluid("copper", 90), fluid("amethyst", 100)], &fluid("alloy", 90)));
        assert_eq!(tank.total(), 270);
    }
    #[test]
    fn selected_fluid_moves_to_bottom_without_changing_amounts() {
        let mut tank = SmelteryTank::new(500);
        assert!(tank.insert("copper", 90)); assert!(tank.insert("amethyst", 100));
        assert!(tank.move_to_bottom(1));
        assert_eq!(tank.fluids, vec![fluid("amethyst", 100), fluid("copper", 90)]);
        assert!(!tank.move_to_bottom(2)); assert_eq!(tank.total(), 190);
    }
}


#[cfg(test)]
mod preservation_tests {
    use super::*;
    fn extended(tank: &SmelteryTank, rows: bool) -> Tag {
        let Tag::Compound(mut fields) = tank.to_nbt() else { unreachable!() };
        fields.insert("future_tank".into(), Tag::Int(7));
        if rows { if let Some(Tag::List(rows)) = fields.get_mut(&"fluids".into()) {
            for (index, row) in rows.iter_mut().enumerate() { if let Tag::Compound(fields) = row {
                fields.insert("future_row".into(), Tag::Int(index as i32));
            }}
        }}
        Tag::Compound(fields)
    }
    #[test]
    fn preserving_rows_follow_exact_tag_identity_and_reordering() {
        let first = Tag::compound([("Potion", Tag::String("minecraft:healing".into()))]);
        let second = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into()))]);
        let mut tank = SmelteryTank::new(4000);
        assert!(tank.insert_tagged("tconstruct:potion", Some(&first), 1500));
        assert!(tank.insert_tagged("tconstruct:potion", Some(&second), 1500));
        let raw = extended(&tank,true);assert_eq!(tank.to_nbt_preserving(&raw),Some(raw.clone()));
        assert!(tank.move_to_bottom(1));assert!(tank.take_tagged("tconstruct:potion",Some(&first),250));
        tank.capacity=2000; // An existing overfull tank can survive a structure shrink.
        let saved=tank.to_nbt_preserving(&raw).unwrap();assert_eq!(SmelteryTank::from_nbt(&saved),Some(tank));
        assert_eq!(saved.get("future_tank"),Some(&Tag::Int(7)));
        let Tag::List(rows)=saved.get("fluids").unwrap() else{panic!()};
        assert_eq!(rows[0].get("future_row"),Some(&Tag::Int(1)));assert_eq!(rows[1].get("future_row"),Some(&Tag::Int(0)));
        assert_eq!(rows[0].get("fluid_tag"),Some(&second));assert_eq!(rows[1].get("fluid_tag"),Some(&first));
    }
    #[test]
    fn preserving_rows_refuse_opaque_removal_and_identity_replacement() {
        let mut tank=SmelteryTank::new(4000);assert!(tank.insert("minecraft:water",1000));
        let raw=extended(&tank,true);let normal=extended(&tank,false);
        assert!(tank.take("minecraft:water",1000));assert!(tank.to_nbt_preserving(&raw).is_none());
        let empty=tank.to_nbt_preserving(&normal).unwrap();assert_eq!(empty.get("future_tank"),Some(&Tag::Int(7)));
        assert!(tank.insert("minecraft:lava",1000));assert!(tank.to_nbt_preserving(&raw).is_none());
        let saved=tank.to_nbt_preserving(&empty).unwrap();assert_eq!(SmelteryTank::from_nbt(&saved),Some(tank));
    }
    #[test]
    fn preserving_rows_refuse_aliases_lossy_ids_and_invalid_owned_state() {
        let mut tank=SmelteryTank::new(4000);assert!(tank.insert("minecraft:water",1000));
        for alias in ["Capacity","Fluids","Fluid","Tank","FluidName","Amount","Tag"] {
            let Tag::Compound(mut fields)=tank.to_nbt() else{panic!()};fields.insert(alias.into(),Tag::Int(1));
            assert!(tank.to_nbt_preserving(&Tag::Compound(fields)).is_none());
        }
        for alias in ["FluidName","Amount","Tag","fluid"] {
            let mut raw=tank.to_nbt();if let Tag::Compound(fields)=&mut raw{if let Some(Tag::List(rows))=fields.get_mut(&"fluids".into()){if let Tag::Compound(row)=&mut rows[0]{row.insert(alias.into(),Tag::Int(1));}}}
            assert!(tank.to_nbt_preserving(&raw).is_none());
        }
        let mut raw=tank.to_nbt();if let Tag::Compound(fields)=&mut raw{if let Some(Tag::List(rows))=fields.get_mut(&"fluids".into()){if let Tag::Compound(row)=&mut rows[0]{row.insert("id".into(),Tag::String(horizon_world::nbt::JavaString(vec![0xd800])));}}}
        let decoded=SmelteryTank::from_nbt(&raw).unwrap();assert!(decoded.to_nbt_preserving(&raw).is_none());
        let raw=tank.to_nbt();tank.fluids.push(tank.fluids[0].clone());assert!(tank.to_nbt_preserving(&raw).is_none());
        tank.fluids.pop();tank.fluids[0].amount=0;assert!(tank.to_nbt_preserving(&raw).is_none());
    }
}
