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
    /// when the remaining tank capacity runs out.
    pub(super) fn foundry_output(&mut self, primary: &FluidAmount, byproducts: &[FluidAmount]) -> bool {
        if primary.tag.is_some() || byproducts.iter().any(|fluid| fluid.tag.is_some()) { return false; }
        let mut next = self.clone();
        if !next.insert(&primary.id, primary.amount) { return false; }
        for fluid in byproducts {
            let amount = fluid.amount.min(next.capacity.saturating_sub(next.total()));
            if amount > 0 { let _ = next.insert(&fluid.id, amount); }
        }
        *self = next;
        true
    }

    /// Commit only when every input and the resulting net volume fit.
    pub(super) fn alloy(&mut self, inputs: &[FluidAmount], result: &FluidAmount) -> bool {
        if inputs.is_empty() || inputs.iter().any(|fluid| fluid.tag.is_some()) || result.tag.is_some() || result.id.is_empty() || result.amount == 0 { return false; }
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
