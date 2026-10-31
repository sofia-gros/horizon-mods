// Standard lava bucket exchange with a source seared tank. The caller commits
// the tank and inventory together and handles the player's creative mode.
use super::tconstruct_source_melter_state::Tank;

pub(super) fn capacity(name: &str) -> Option<u32> {
    match name {
        "tconstruct:seared_fuel_tank" | "tconstruct:seared_fuel_gauge" => Some(4000),
        "tconstruct:seared_ingot_tank" | "tconstruct:seared_ingot_gauge" => Some(48 * 90),
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
