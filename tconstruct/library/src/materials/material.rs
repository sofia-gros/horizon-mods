//! `library.materials.definition.IMaterial` and `Material`.
use std::cmp::Ordering;

use super::ids::MaterialId;

/// Minecraft's `Rarity`; the JSON writes it in lower case.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
}

impl Rarity {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "common" => Some(Self::Common),
            "uncommon" => Some(Self::Uncommon),
            "rare" => Some(Self::Rare),
            "epic" => Some(Self::Epic),
            _ => None,
        }
    }

    /// `IMaterial.computeRarity`: from tier 3 up.
    pub fn for_tier(tier: i32) -> Self {
        match tier {
            t if t >= 5 => Self::Epic,
            4 => Self::Rare,
            3 => Self::Uncommon,
            _ => Self::Common,
        }
    }
}

/// A material: its id, tier, where it sorts, whether a player can craft parts from it and whether it is hidden from lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Material {
    pub id: MaterialId,
    pub tier: i32,
    pub sort_order: i32,
    pub rarity: Rarity,
    pub craftable: bool,
    pub hidden: bool,
}

impl Material {
    /// `IMaterial.UNKNOWN`: not craftable, hidden, tier 0, sort order -1.
    pub fn unknown() -> Self {
        Self { id: MaterialId::unknown(), tier: 0, sort_order: -1, rarity: Rarity::Common, craftable: false, hidden: true }
    }
}

impl PartialOrd for Material {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// `IMaterial.compareTo`: tier first, then sort order, then the id.
impl Ord for Material {
    fn cmp(&self, other: &Self) -> Ordering {
        self.tier.cmp(&other.tier).then(self.sort_order.cmp(&other.sort_order)).then_with(|| self.id.cmp(&other.id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn material(path: &str, tier: i32, sort_order: i32) -> Material {
        Material { id: MaterialId::try_new("tconstruct", path).unwrap(), tier, sort_order, rarity: Rarity::for_tier(tier), craftable: true, hidden: false }
    }

    #[test]
    fn rarity_comes_from_the_tier() {
        assert_eq!(Rarity::for_tier(0), Rarity::Common);
        assert_eq!(Rarity::for_tier(2), Rarity::Common);
        assert_eq!(Rarity::for_tier(3), Rarity::Uncommon);
        assert_eq!(Rarity::for_tier(4), Rarity::Rare);
        assert_eq!(Rarity::for_tier(5), Rarity::Epic);
        assert_eq!(Rarity::for_tier(9), Rarity::Epic);
    }

    #[test]
    fn materials_sort_by_tier_then_order_then_id() {
        let mut all = vec![material("b", 2, 1), material("a", 2, 1), material("c", 1, 9), material("d", 2, 0)];
        all.sort();
        let names: Vec<&str> = all.iter().map(|m| m.id.path()).collect();
        assert_eq!(names, ["c", "d", "a", "b"]);
    }
}
