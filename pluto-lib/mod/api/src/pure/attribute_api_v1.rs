//! Attributes a mod registers itself, for the ones the host does not have.
//!
//! The host's attribute set is fixed: a mod can put modifiers for the host's own attributes on a
//! stack and the game applies them, but an attribute of the mod's own — Tinkers' bouncy, jump
//! count, experience multiplier and their kin — has no host counterpart, so a stack entry naming
//! one would be data nothing applies. This module is the registry and the arithmetic for those:
//! a mod declares an attribute (its id, its default, the range its value is clamped to), the
//! mod's own flows collect the contributions its equipment, tools and effects make to it, and
//! [`attribute_value`] works the value out with the game's own modifier arithmetic, for the
//! mod's flows to read where the original reads the attribute.
//!
//! Nothing here applies a value to an entity: the host cannot be told about an attribute it
//! lacks. A mod keeps the values it works out wherever its readers can reach them (Tinkers keeps
//! them per player from the player tick), and a contribution to one of the host's own attributes
//! still goes through the host's machinery (a stack entry, or the player tick's base additions).

use std::collections::BTreeMap;
use std::sync::RwLock;

use super::base::Identifier;

/// How one contribution bends an attribute's value, in the data's own names (the game's three
/// modifier operations). The order the operations apply in is fixed by [`attribute_value`]:
/// every addition first, then every multiply-base against the added value, then each
/// multiply-total in turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttributeOperation {
    /// The amount is added to the value (`addition`).
    Addition,
    /// The amount, times the value after additions, is added (`multiply_base`).
    MultiplyBase,
    /// The value is multiplied by one plus the amount, one such contribution at a time
    /// (`multiply_total`).
    MultiplyTotal,
}

impl AttributeOperation {
    /// The operation the data names, or None for a name that is not one of the three.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "addition" => Some(Self::Addition),
            "multiply_base" => Some(Self::MultiplyBase),
            "multiply_total" => Some(Self::MultiplyTotal),
            _ => None,
        }
    }

    /// The name the data uses for this operation.
    pub fn name(self) -> &'static str {
        match self {
            Self::Addition => "addition",
            Self::MultiplyBase => "multiply_base",
            Self::MultiplyTotal => "multiply_total",
        }
    }
}

/// One attribute a mod declares: its id, the value an entity has with no contributions, and
/// the range the worked-out value is clamped into.
#[derive(Clone, Debug, PartialEq)]
pub struct AttributeDef {
    /// The attribute's id, such as `tconstruct:generic.bouncy`.
    pub id: Identifier,
    /// The value with no contributions (the original's registration default).
    pub default: f64,
    /// The lowest value the attribute may take.
    pub min: f64,
    /// The highest value the attribute may take.
    pub max: f64,
}

/// One amount contributed to an attribute: by an equipment or tool module on a worn or held
/// stack, by a max-armor claim, or by an effect active on the entity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttributeContribution {
    /// How the amount bends the value.
    pub operation: AttributeOperation,
    /// The amount, already scaled by whatever level produced it (a module's formula result, an
    /// effect's per-level amount times its level).
    pub amount: f64,
}

impl AttributeContribution {
    /// This contribution scaled by a count, as an effect's modifier is scaled by its level (the
    /// game's effect attribute modifiers grow by the per-level amount times amplifier + 1).
    pub fn scaled(self, count: i32) -> Self {
        Self { operation: self.operation, amount: self.amount * f64::from(count) }
    }
}

/// An attribute's value under `contributions`, worked out as the game works out an attribute
/// instance: the default, plus every addition; that sum grown by each multiply-base amount's
/// share of it; the result then multiplied by one plus each multiply-total amount in turn; and
/// the whole clamped into the declared range. A contribution whose amount is not finite is
/// ignored, and a result that is not finite falls back to the default.
pub fn attribute_value(def: &AttributeDef, contributions: &[AttributeContribution]) -> f64 {
    let mut added = def.default;
    for contribution in contributions {
        if contribution.operation == AttributeOperation::Addition && contribution.amount.is_finite() {
            added += contribution.amount;
        }
    }
    let mut value = added;
    for contribution in contributions {
        if contribution.operation == AttributeOperation::MultiplyBase && contribution.amount.is_finite() {
            value += added * contribution.amount;
        }
    }
    for contribution in contributions {
        if contribution.operation == AttributeOperation::MultiplyTotal && contribution.amount.is_finite() {
            value *= 1.0 + contribution.amount;
        }
    }
    if !value.is_finite() {
        return def.default;
    }
    value.clamp(def.min, def.max)
}

static ATTRIBUTES: RwLock<BTreeMap<String, AttributeDef>> = RwLock::new(BTreeMap::new());

/// The attributes mods have declared, by id. Registration is process-lifetime and registering
/// an id again replaces its declaration.
pub struct AttributeRegistry;

impl AttributeRegistry {
    /// Declares an attribute. The declaration is validated whole first — the numbers finite and
    /// the default inside the range — so a refused declaration leaves any previous one intact.
    pub fn register(def: AttributeDef) -> Result<(), String> {
        if !def.default.is_finite() || !def.min.is_finite() || !def.max.is_finite() {
            return Err(format!("attribute {} numbers must be finite", def.id));
        }
        if def.min > def.max || !(def.min..=def.max).contains(&def.default) {
            return Err(format!("attribute {} default {} outside its range {}..={}", def.id, def.default, def.min, def.max));
        }
        let mut attributes = ATTRIBUTES.write().map_err(|_| "attribute registry poisoned")?;
        attributes.insert(def.id.to_string(), def);
        Ok(())
    }

    /// The declaration of the attribute `id`, or None when no mod declared one.
    pub fn get(id: &str) -> Option<AttributeDef> {
        ATTRIBUTES.read().ok()?.get(id).cloned()
    }

    /// Whether a mod declared the attribute `id`.
    pub fn is_registered(id: &str) -> bool {
        ATTRIBUTES.read().is_ok_and(|attributes| attributes.contains_key(id))
    }

    /// Every declared attribute, sorted by id (the registry's own census, for tests and tools).
    pub fn registered() -> Vec<AttributeDef> {
        ATTRIBUTES.read().map(|attributes| attributes.values().cloned().collect()).unwrap_or_default()
    }

    /// The value of the declared attribute `id` under `contributions`, or None when no mod
    /// declared it.
    pub fn value(id: &str, contributions: &[AttributeContribution]) -> Option<f64> {
        Self::get(id).map(|def| attribute_value(&def, contributions))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def() -> AttributeDef {
        AttributeDef { id: Identifier::parse("test:multiplier").unwrap(), default: 1.0, min: 0.0, max: 100.0 }
    }

    fn contribution(operation: AttributeOperation, amount: f64) -> AttributeContribution {
        AttributeContribution { operation, amount }
    }

    #[test]
    fn operations_round_trip_through_their_data_names() {
        for operation in [AttributeOperation::Addition, AttributeOperation::MultiplyBase, AttributeOperation::MultiplyTotal] {
            assert_eq!(AttributeOperation::from_name(operation.name()), Some(operation));
        }
        assert_eq!(AttributeOperation::from_name("add_value"), None, "the stack entry name is not the data name");
    }

    #[test]
    fn the_value_follows_the_games_modifier_arithmetic() {
        // Additions land first, multiply-base reads the added value, multiply-total compounds last.
        let contributions = [
            contribution(AttributeOperation::MultiplyTotal, 1.0),
            contribution(AttributeOperation::MultiplyBase, 0.25),
            contribution(AttributeOperation::Addition, 3.0),
        ];
        // (1 + 3) grown by a quarter of 4 is 5, doubled is 10.
        assert_eq!(attribute_value(&def(), &contributions), 10.0);
        assert_eq!(attribute_value(&def(), &[]), 1.0, "no contributions, the default");
    }

    #[test]
    fn the_value_is_clamped_into_the_declared_range() {
        let contributions = [contribution(AttributeOperation::Addition, 1000.0)];
        assert_eq!(attribute_value(&def(), &contributions), 100.0);
        let contributions = [contribution(AttributeOperation::Addition, -1000.0)];
        assert_eq!(attribute_value(&def(), &contributions), 0.0);
    }

    #[test]
    fn a_registered_attribute_answers_its_value_and_an_unregistered_one_none() {
        AttributeRegistry::register(def()).unwrap();
        assert!(AttributeRegistry::is_registered("test:multiplier"));
        assert_eq!(AttributeRegistry::value("test:multiplier", &[contribution(AttributeOperation::Addition, 1.0)]), Some(2.0));
        assert_eq!(AttributeRegistry::value("test:missing", &[]), None);
        assert!(AttributeRegistry::register(AttributeDef { default: 5.0, ..def() }).is_err(), "a default outside the range is refused");
        assert_eq!(AttributeRegistry::get("test:multiplier").unwrap().default, 1.0, "the refused registration changed nothing");
    }
}
