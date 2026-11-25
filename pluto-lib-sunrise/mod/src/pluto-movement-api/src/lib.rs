//! Block movement factors for mod blocks: the speed and jump factors an entity
//! moving on the block uses, as `BlockBehaviour.Properties.speedFactor` and
//! `jumpFactor` set them in the original. The host's block declarations carry no
//! such key (their `BlockProperties` denies unknown fields), so the factors ride
//! here and the `pluto_movement` mixin applies them to the host's movement table
//! when it loads. Register before the first movement-table load; a registration
//! made after a load applies on the next one.
use std::{
    collections::BTreeMap,
    sync::{OnceLock, RwLock},
};

/// The two movement factors of one block: `speed_factor` scales an entity's
/// movement speed on the block, `jump_factor` its jump height. Both are 1.0 in
/// the original and on the host unless a block sets them; only non-default pairs
/// need registering.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MovementFactors {
    pub speed_factor: f32,
    pub jump_factor: f32,
}

static REGISTRY: OnceLock<RwLock<BTreeMap<String, MovementFactors>>> = OnceLock::new();
fn registry() -> &'static RwLock<BTreeMap<String, MovementFactors>> {
    REGISTRY.get_or_init(Default::default)
}

fn valid_id(id: &str) -> bool {
    let Some((namespace, path)) = id.split_once(':') else {
        return false;
    };
    !namespace.is_empty() && !path.is_empty() && id.matches(':').count() == 1
}

/// Register the movement factors of one block. Registering the same block with
/// the same factors again is a no-op; registering it with different factors is
/// an error, so two mods cannot silently disagree about a block.
pub fn register(block: &str, speed_factor: f32, jump_factor: f32) -> Result<(), &'static str> {
    if !valid_id(block) {
        return Err("expected a namespaced block ID");
    }
    if !speed_factor.is_finite() || speed_factor < 0.0 {
        return Err("speed_factor must be finite and 0 or more");
    }
    if !jump_factor.is_finite() || jump_factor < 0.0 {
        return Err("jump_factor must be finite and 0 or more");
    }
    let factors = MovementFactors { speed_factor, jump_factor };
    let mut entries = registry().write().map_err(|_| "movement registry poisoned")?;
    match entries.get(block) {
        Some(previous) if *previous == factors => Ok(()),
        Some(_) => Err("block movement already registered with different factors"),
        None => {
            entries.insert(block.to_owned(), factors);
            Ok(())
        }
    }
}

/// The factors registered for a block, if any. A block with no registration
/// keeps its template's factors, which are 1.0 and 1.0 for the vanilla templates.
pub fn get(block: &str) -> Option<MovementFactors> {
    registry().read().ok().and_then(|entries| entries.get(block).copied())
}

/// Every registration, in block-ID order, for the movement-table injection.
pub fn registered() -> Vec<(String, MovementFactors)> {
    registry()
        .read()
        .map(|entries| entries.iter().map(|(block, factors)| (block.clone(), *factors)).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_re_registration_is_a_noop_and_conflicts_are_rejected() {
        register("test:soul_glass", 0.1, 1.0).unwrap();
        register("test:soul_glass", 0.1, 1.0).unwrap();
        assert_eq!(get("test:soul_glass"), Some(MovementFactors { speed_factor: 0.1, jump_factor: 1.0 }));
        assert!(register("test:soul_glass", 0.2, 1.0).is_err());
        assert!(register("test:soul_glass", 0.1, 0.5).is_err());
        assert_eq!(get("test:soul_glass"), Some(MovementFactors { speed_factor: 0.1, jump_factor: 1.0 }));
    }

    #[test]
    fn rejects_bad_ids_and_factors() {
        assert!(register("soul_glass", 0.1, 1.0).is_err());
        assert!(register("test:", 0.1, 1.0).is_err());
        assert!(register(":soul_glass", 0.1, 1.0).is_err());
        assert!(register("test:soul_glass", f32::NAN, 1.0).is_err());
        assert!(register("test:soul_glass", -0.1, 1.0).is_err());
        assert!(register("test:soul_glass", 0.1, f32::INFINITY).is_err());
        assert_eq!(get("test:unknown"), None);
    }

    #[test]
    fn registered_lists_blocks_in_id_order() {
        register("test:zeta", 0.4, 0.5).unwrap();
        register("test:alpha", 0.2, 1.0).unwrap();
        let listed: Vec<&str> = registered().iter().map(|(block, _)| block.as_str()).collect();
        assert!(listed.windows(2).all(|pair| pair[0] < pair[1]), "sorted by block ID: {listed:?}");
        assert_eq!(get("test:alpha"), Some(MovementFactors { speed_factor: 0.2, jump_factor: 1.0 }));
    }
}
