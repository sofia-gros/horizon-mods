//! Fluid amounts and the melting/cooling arithmetic of the smeltery domain, from SlimeKnights FluidValues,
//! IMeltingRecipe, IMeltingContainer.OreRateType, Config.OreRate and DamageableMeltingRecipe (MIT; see LICENSE).
//! Pure rules only: nothing here is connected to a host fluid or block entity yet.
#![allow(dead_code)]

// Amounts are in the original's units (a bucket is 1000).
pub(crate) const BUCKET: i32 = 1000;
pub(crate) const INGOT: i32 = 90;
pub(crate) const NUGGET: i32 = INGOT / 9;
pub(crate) const METAL_BLOCK: i32 = INGOT * 9;
pub(crate) const GEM: i32 = 100;
pub(crate) const GEM_SHARD: i32 = GEM / 4;
pub(crate) const LARGE_GEM_BLOCK: i32 = GEM * 9;
pub(crate) const SMALL_GEM_BLOCK: i32 = GEM * 4;
pub(crate) const BRICK: i32 = 250;
pub(crate) const BRICK_BLOCK: i32 = BRICK * 4;
pub(crate) const GLASS_BLOCK: i32 = 1000;
pub(crate) const GLASS_PANE: i32 = GLASS_BLOCK / 4;
pub(crate) const SLIMEBALL: i32 = 250;
pub(crate) const SLIME_CONGEALED: i32 = SLIMEBALL * 4;
pub(crate) const SLIME_BLOCK: i32 = SLIMEBALL * 9;
pub(crate) const SLIME_DROP: i32 = SLIMEBALL / 5;
pub(crate) const ICHOR_BYPRODUCT: i32 = SLIME_DROP * 4;
pub(crate) const ICHOR_BLAZING_BLOOD: i32 = BUCKET / 100;
pub(crate) const BOWL: i32 = 250;
pub(crate) const BOTTLE: i32 = 250;
pub(crate) const SIP: i32 = BOTTLE / 5;
pub(crate) const LANTERN_CAPACITY: i32 = 50;

/// Fluid temperatures are kelvin; recipes and fuels use celsius (IMeltingRecipe.getTemperature).
pub(crate) fn celsius(kelvin: i32) -> i32 { kelvin - 300 }

/// IMeltingRecipe.calcTime: `temperature^0.585 * factor` in 1/5th seconds, rounded. The exponent is the float
/// literal 0.585f widened to double, as Java evaluates `Math.pow(temperature + 300, 0.585f)`.
pub(crate) fn calc_time(temperature: i32, factor: f32) -> i32 {
    let value = f64::from(temperature + 300).powf(f64::from(0.585f32)) * f64::from(factor);
    // Math.round is floor(x + 0.5)
    (value + 0.5).floor() as i32
}

/// IMeltingRecipe.calcTimeFactor: the square root of how many ingots the amount is (float arithmetic).
pub(crate) fn calc_time_factor(amount: i32) -> f32 {
    f64::from(amount as f32 / INGOT as f32).sqrt() as f32
}

/// IMeltingRecipe.calcTimeForAmount, which ICastingRecipe.calcCoolingTime reuses for the casting time.
pub(crate) fn calc_time_for_amount(temperature: i32, amount: i32) -> i32 { calc_time(temperature, calc_time_factor(amount)) }
pub(crate) fn calc_cooling_time(temperature: i32, amount: i32) -> i32 { calc_time_for_amount(temperature, amount) }

/// IMeltingContainer.OreRateType.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OreRateType { None, Metal, Gem, Default }
impl OreRateType {
    pub(crate) fn parse(text: &str) -> Option<Self> {
        match text {
            "none" => Some(Self::None), "metal" => Some(Self::Metal), "gem" => Some(Self::Gem), "default" => Some(Self::Default),
            _ => None,
        }
    }
    /// orElse: DEFAULT defers to the other rate.
    pub(crate) fn or_else(self, other: Self) -> Self { if self == Self::Default { other } else { self } }
}

/// Config.OreRate: nuggets per metal ore unit (a value of 9 is one ingot) and gem shards per gem ore unit (4 is one gem).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OreRate { pub nuggets_per_metal: i32, pub shards_per_gem: i32 }
impl OreRate {
    /// The defaults of the melter, smeltery, foundry and the foundry's byproducts (Config.COMMON).
    pub(crate) const MELTER: Self = Self { nuggets_per_metal: 12, shards_per_gem: 8 };
    pub(crate) const SMELTERY: Self = Self { nuggets_per_metal: 12, shards_per_gem: 8 };
    pub(crate) const FOUNDRY: Self = Self { nuggets_per_metal: 9, shards_per_gem: 4 };
    pub(crate) const FOUNDRY_BYPRODUCT: Self = Self { nuggets_per_metal: 3, shards_per_gem: 4 };

    /// applyOreBoost: integer arithmetic, so `amount * rate / 9` truncates toward zero.
    pub(crate) fn apply(&self, rate: OreRateType, amount: i32) -> i32 {
        match rate {
            OreRateType::Metal => amount.wrapping_mul(self.nuggets_per_metal) / 9,
            OreRateType::Gem => amount.wrapping_mul(self.shards_per_gem) / 4,
            OreRateType::None | OreRateType::Default => amount,
        }
    }
}

/// DamageableMeltingRecipe.scaleOutput: the amount scales with the undamaged share of the item; the minimum is one
/// unit and a larger amount is rounded down to a whole number of units.
pub(crate) fn scale_output(amount: i32, damage: i32, max_damage: i32, unit_size: i32) -> i32 {
    let mut amount = amount.wrapping_mul(max_damage - damage) / max_damage;
    if amount <= unit_size {
        amount = unit_size.max(1);
    } else if unit_size > 1 {
        let remainder = amount % unit_size;
        if remainder > 0 { amount -= remainder; }
    }
    amount
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_amounts_derive_from_the_ingot() {
        assert_eq!((NUGGET, METAL_BLOCK, GEM_SHARD, LARGE_GEM_BLOCK, SMALL_GEM_BLOCK), (10, 810, 25, 900, 400));
        assert_eq!((SLIME_DROP, ICHOR_BYPRODUCT, ICHOR_BLAZING_BLOOD, SIP, BRICK_BLOCK), (50, 200, 10, 50, 1000));
    }

    #[test]
    fn the_float_exponent_and_half_up_rounding_follow_java() {
        // 1100^0.585 = 60.1: an iron ingot at 800 C takes 60. A block is sqrt(9) times the factor.
        assert_eq!(calc_time_for_amount(800, INGOT), 60);
        assert_eq!(calc_time_for_amount(800, METAL_BLOCK), 180);
        assert_eq!(calc_time_factor(INGOT), 1.0);
        assert_eq!(calc_time(0, 0.0), 0);
        // an amount that is not a whole ingot still takes the square root of its ingot share
        assert_eq!(calc_time_factor(10), (10f32 / 90f32).sqrt());
    }

    #[test]
    fn ore_rates_truncate_and_default_defers() {
        assert_eq!(OreRate::SMELTERY.apply(OreRateType::Metal, INGOT), 120, "12 nuggets per ore from a 90 base");
        assert_eq!(OreRate::SMELTERY.apply(OreRateType::Gem, GEM), 200);
        assert_eq!(OreRate::FOUNDRY_BYPRODUCT.apply(OreRateType::Metal, 10), 3, "10 * 3 / 9 truncates");
        assert_eq!(OreRate::MELTER.apply(OreRateType::Default, 77), 77);
        assert_eq!(OreRateType::Default.or_else(OreRateType::Gem), OreRateType::Gem);
        assert_eq!(OreRateType::Metal.or_else(OreRateType::Gem), OreRateType::Metal);
    }

    #[test]
    fn damaged_items_give_whole_units_and_at_least_one() {
        // a 300 mB diamond tool in 25 mB units, half worn: 150, already whole units
        assert_eq!(scale_output(300, 50, 100, 25), 150);
        // 300 * 33 / 100 = 99, rounded down to 75
        assert_eq!(scale_output(300, 67, 100, 25), 75);
        // nearly broken: the floor is one unit
        assert_eq!(scale_output(300, 99, 100, 25), 25);
        assert_eq!(scale_output(300, 100, 100, 25), 25);
        assert_eq!(scale_output(300, 100, 100, 1), 1);
        assert_eq!(scale_output(300, 0, 100, 1), 300);
    }
}
