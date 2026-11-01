// Standard lava bucket exchange with a source seared tank. The caller commits
// the tank and inventory together and handles the player's creative mode.
use super::tconstruct_source_melter_state::Tank;

pub(super) fn capacity(name: &str) -> Option<u32> {
    match name {
        "tconstruct:seared_fuel_tank" | "tconstruct:seared_fuel_gauge" | "tconstruct:scorched_fuel_tank" | "tconstruct:scorched_fuel_gauge" => Some(4000),
        "tconstruct:seared_ingot_tank" | "tconstruct:seared_ingot_gauge" | "tconstruct:scorched_ingot_tank" | "tconstruct:scorched_ingot_gauge" => Some(48 * 90),
        _ => None,
    }
}

/// Return the replacement container after a complete bucket transfer. The
/// caller must perform this on a copy until both inventory and NBT can commit.
pub(super) fn exchange(tank: &mut Tank, held: &str) -> Option<&'static str> {
    match held {
        "minecraft:lava_bucket" if tank.insert("minecraft:lava", 1000) => Some("minecraft:bucket"),
        "minecraft:bucket" if tank.fluid.as_deref() == Some("minecraft:lava") && tank.amount >= 1000 => {
            tank.take(1000)?; Some("minecraft:lava_bucket")
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bucket_exchange_is_all_or_nothing() {
        let mut tank = Tank::new(capacity("tconstruct:seared_fuel_tank").unwrap());
        assert_eq!(exchange(&mut tank, "minecraft:lava_bucket"), Some("minecraft:bucket"));
        assert_eq!(tank.amount, 1000);
        assert_eq!(exchange(&mut tank, "minecraft:bucket"), Some("minecraft:lava_bucket"));
        assert_eq!(tank.amount, 0);
        assert_eq!(exchange(&mut tank, "minecraft:bucket"), None);
        assert!(tank.insert("minecraft:lava", 3500));
        assert_eq!(exchange(&mut tank, "minecraft:lava_bucket"), None);
        assert_eq!(tank.amount, 3500);
    }
    #[test]
    fn source_tank_capacities_are_distinct() {
        assert_eq!(capacity("tconstruct:seared_fuel_gauge"), Some(4000));
        assert_eq!(capacity("tconstruct:seared_ingot_gauge"), Some(4320));
        assert_eq!(capacity("minecraft:glass"), None);
    }
}

/// GUI-only snapshot of the source fuel module's selected fluid tanks.
#[derive(Clone, Debug, Default)]
pub(super) struct FuelView {
    pub last: Option<(i32, i32, i32)>,
    pub has_tank: bool,
    pub fluid: Option<String>,
    pub amount: u32,
    pub capacity: u32,
    pub temperature: u32,
}
impl FuelView {
    pub(super) fn collect(tanks: &[((i32, i32, i32), Tank)], last: Option<(i32, i32, i32)>, temperature: impl Fn(&str) -> u32) -> Self {
        let primary = last.and_then(|pos| tanks.iter().find(|(at, tank)| *at == pos && tank.fluid.is_some()))
            .or_else(|| tanks.iter().find(|(_, tank)| tank.fluid.is_some()));
        let Some((pos, tank)) = primary else { return Self { has_tank: !tanks.is_empty(), ..Self::default() }; };
        let Some(fluid) = tank.fluid.clone() else { return Self::default(); };
        let mut view = Self { last: Some(*pos), has_tank: true, temperature: temperature(&fluid), fluid: Some(fluid.clone()), amount: 0, capacity: 0 };
        for (_, tank) in tanks {
            if tank.fluid.is_none() || tank.fluid.as_deref() == Some(fluid.as_str()) {
                view.amount = view.amount.saturating_add(tank.amount);
                view.capacity = view.capacity.saturating_add(tank.capacity);
            }
        }
        view
    }
    pub(super) fn to_nbt(&self) -> horizon_world::nbt::Tag {
        use horizon_world::nbt::Tag;
        let mut fields = horizon_world::nbt::Compound::new();
        fields.insert("has_tank".into(), Tag::Byte(i8::from(self.has_tank)));
        if let Some((x,y,z)) = self.last { fields.insert("last".into(), Tag::IntArray(vec![x,y,z])); }
        if let Some(id) = &self.fluid { fields.insert("fluid".into(), Tag::String(id.as_str().into())); }
        for (name, value) in [("amount", self.amount), ("capacity", self.capacity), ("temperature", self.temperature)] {
            fields.insert(name.into(), Tag::Long(i64::from(value)));
        }
        Tag::Compound(fields)
    }
    pub(super) fn from_nbt(tag: &horizon_world::nbt::Tag) -> Option<Self> {
        use horizon_world::nbt::Tag;
        let number = |name| match tag.get(name)? { Tag::Long(value) => u32::try_from(*value).ok(), _ => None };
        let last = match tag.get("last") { Some(Tag::IntArray(v)) if v.len() == 3 => Some((v[0],v[1],v[2])), _ => None };
        let fluid = match tag.get("fluid") { Some(Tag::String(id)) => Some(id.to_string_lossy()), _ => None };
        Some(Self { last, has_tank: matches!(tag.get("has_tank"), Some(Tag::Byte(1))), fluid,
            amount: number("amount")?, capacity: number("capacity")?, temperature: number("temperature")? })
    }
}

#[cfg(test)]
mod view_tests {
    use super::*;
    #[test] fn source_view_prefers_last_nonempty_and_combines_same_or_empty_tanks() {
        let mut a = Tank::new(4000); a.insert("minecraft:lava", 1000);
        let mut b = Tank::new(4000); b.insert("minecraft:water", 1000);
        let empty = Tank::new(4320);
        let tanks = vec![((0,0,0),a), ((1,0,0),b), ((2,0,0),empty)];
        let view = FuelView::collect(&tanks, Some((1,0,0)), |_| 500);
        assert_eq!((view.fluid.as_deref(), view.amount, view.capacity, view.temperature), (Some("minecraft:water"), 1000, 8320, 500));
        let restored = FuelView::from_nbt(&view.to_nbt()).unwrap();
        assert_eq!((restored.last, restored.fluid, restored.capacity), (Some((1,0,0)), Some("minecraft:water".into()), 8320));
    }
}
