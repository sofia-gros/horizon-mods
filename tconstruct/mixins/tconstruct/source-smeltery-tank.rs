// Multi-fluid storage and atomic alloy operations for a Smeltery.
//
// Recipe tags and conditions are resolved by the caller against loaded data.
// This module does not select recipes or alter a world by itself.

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FluidAmount {
    pub id: String,
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
        self.fluids.iter().find(|f| f.id == id).map_or(0, |f| f.amount)
    }

    pub(super) fn insert(&mut self, id: &str, amount: u32) -> bool {
        if id.is_empty() || amount == 0 || amount > self.capacity.saturating_sub(self.total()) { return false; }
        if let Some(fluid) = self.fluids.iter_mut().find(|f| f.id == id) {
            fluid.amount += amount;
        } else {
            self.fluids.push(FluidAmount { id: id.to_owned(), amount });
        }
        true
    }

    pub(super) fn take(&mut self, id: &str, amount: u32) -> bool {
        if amount == 0 { return false; }
        let Some(index) = self.fluids.iter().position(|f| f.id == id && f.amount >= amount) else { return false; };
        self.fluids[index].amount -= amount;
        if self.fluids[index].amount == 0 { self.fluids.remove(index); }
        true
    }

    /// Commit only when every input and the resulting net volume fit.
    pub(super) fn alloy(&mut self, inputs: &[FluidAmount], result: &FluidAmount) -> bool {
        if inputs.is_empty() || result.id.is_empty() || result.amount == 0 { return false; }
        let mut next = self.clone();
        for input in inputs {
            if !next.take(&input.id, input.amount) { return false; }
        }
        if !next.insert(&result.id, result.amount) { return false; }
        *self = next;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fluid(id: &str, amount: u32) -> FluidAmount { FluidAmount { id: id.into(), amount } }
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
}
