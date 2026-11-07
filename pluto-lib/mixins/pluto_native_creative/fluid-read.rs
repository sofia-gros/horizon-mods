// Read-only access to the host's registered block-entity fluid tanks.
use crate::{containers::BlockEntity, world::World};
use horizon_core::geometry::Direction;
use horizon_world::nbt::Tag;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FluidTank {
    pub fluid: Option<String>,
    pub amount: i32,
    pub capacity: i32,
}

/// The first tank exposed on `face`, using the host's native tank registry
/// and persisted `fluids` representation. No fluid is inserted or removed.
pub fn first(world: &World, pos: (i32, i32, i32), face: Direction) -> Option<FluidTank> {
    if !world.loaded(pos.0, pos.2) {
        return None;
    }
    let state = world.block(pos.0, pos.1, pos.2);
    let tanks = world.block_entities.mod_fluids(state.0)?;
    let index = *tanks.faces[face as usize].first()?;
    let capacity = *tanks.tanks.get(index)?;
    let BlockEntity::Mod(entity) = world.block_entities.get(&pos)? else {
        return None;
    };
    if entity.id.is_empty() {
        return None;
    }
    let saved = match entity.nbt.get(&"fluids".into()) {
        Some(Tag::List(values)) => values.get(index),
        _ => None,
    };
    let fluid = match saved.and_then(|value| value.get("Fluid")) {
        Some(Tag::String(id)) if id.to_string_lossy() != "minecraft:empty" => {
            Some(id.to_string_lossy())
        }
        _ => None,
    };
    let amount = match saved.and_then(|value| value.get("Amount")) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    };
    Some(FluidTank {
        fluid,
        amount,
        capacity,
    })
}

/// Validated view of the native first-tank projection. This is read-only data,
/// not a writable fluid capability or proof that all saved NBT is understood.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TankSnapshot {
    pub fluid: Option<String>,
    pub amount: u32,
    pub capacity: u32,
    /// False for this native projection: omitted metadata is not an empty tag.
    pub metadata_complete: bool,
}

/// An exposed native tank whose projected contents are not safe to display.
/// This is distinct from a missing provider: callers must not silently fall
/// back to another provider after observing an invalid tank.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvalidTank {
    NonPositiveCapacity(i32),
    NegativeAmount(i32),
    OverCapacity { amount: u32, capacity: u32 },
    MissingFluid,
}
impl std::fmt::Display for InvalidTank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonPositiveCapacity(value) => {
                write!(f, "tank capacity must be positive, got {value}")
            }
            Self::NegativeAmount(value) => {
                write!(f, "tank amount must be nonnegative, got {value}")
            }
            Self::OverCapacity { amount, capacity } => {
                write!(f, "tank amount {amount} exceeds capacity {capacity}")
            }
            Self::MissingFluid => f.write_str("nonempty tank has no fluid identity"),
        }
    }
}
impl std::error::Error for InvalidTank {}

impl TryFrom<FluidTank> for TankSnapshot {
    type Error = InvalidTank;
    fn try_from(raw: FluidTank) -> Result<Self, Self::Error> {
        if raw.capacity <= 0 {
            return Err(InvalidTank::NonPositiveCapacity(raw.capacity));
        }
        let capacity = raw.capacity as u32;
        let amount =
            u32::try_from(raw.amount).map_err(|_| InvalidTank::NegativeAmount(raw.amount))?;
        if amount > capacity {
            return Err(InvalidTank::OverCapacity { amount, capacity });
        }
        let fluid = if amount == 0 {
            None
        } else {
            Some(raw.fluid.ok_or(InvalidTank::MissingFluid)?)
        };
        Ok(Self {
            fluid,
            amount,
            capacity,
            metadata_complete: false,
        })
    }
}

/// Read the first exposed tank once and validate its projected capacity/content.
/// `Ok(None)` retains `first`'s unavailable/not-exposed outcome. `Err` means an
/// exposed projection failed validation. No extra provider probing is performed.
/// The legacy `first` reader, including its missing-row defaults, is unchanged.
pub fn first_snapshot(
    world: &World,
    pos: (i32, i32, i32),
    face: Direction,
) -> Result<Option<TankSnapshot>, InvalidTank> {
    first(world, pos, face)
        .map(TankSnapshot::try_from)
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn raw(fluid: Option<&str>, amount: i32, capacity: i32) -> FluidTank {
        FluidTank {
            fluid: fluid.map(str::to_owned),
            amount,
            capacity,
        }
    }

    #[test]
    fn snapshot_preserves_valid_contents_and_marks_metadata_omitted() {
        let original = raw(Some("example:fluid"), i32::MAX, i32::MAX);
        let snapshot = TankSnapshot::try_from(original.clone()).unwrap();
        assert_eq!(snapshot.fluid.as_deref(), Some("example:fluid"));
        assert_eq!(snapshot.amount, i32::MAX as u32);
        assert_eq!(snapshot.capacity, i32::MAX as u32);
        assert!(!snapshot.metadata_complete);
        assert_eq!(original, raw(Some("example:fluid"), i32::MAX, i32::MAX));
    }

    #[test]
    fn zero_amount_normalizes_stale_identity_without_claiming_known_metadata() {
        for fluid in [None, Some("example:fluid")] {
            let snapshot = TankSnapshot::try_from(raw(fluid, 0, 1)).unwrap();
            assert_eq!(snapshot.fluid, None);
            assert_eq!(snapshot.amount, 0);
            assert_eq!(snapshot.capacity, 1);
            assert!(!snapshot.metadata_complete);
        }
    }

    #[test]
    fn invalid_exposed_projection_is_not_a_missing_provider() {
        for capacity in [i32::MIN, -1, 0] {
            assert_eq!(
                TankSnapshot::try_from(raw(None, 0, capacity)),
                Err(InvalidTank::NonPositiveCapacity(capacity))
            );
        }
        assert_eq!(
            TankSnapshot::try_from(raw(None, -1, 1000)),
            Err(InvalidTank::NegativeAmount(-1))
        );
        assert_eq!(
            TankSnapshot::try_from(raw(Some("example:fluid"), 1001, 1000)),
            Err(InvalidTank::OverCapacity {
                amount: 1001,
                capacity: 1000
            })
        );
        assert_eq!(
            TankSnapshot::try_from(raw(None, 1, 1000)),
            Err(InvalidTank::MissingFluid)
        );
        let absent: Option<FluidTank> = None;
        assert_eq!(absent.map(TankSnapshot::try_from).transpose(), Ok(None));
        let invalid = Some(raw(None, 1, 1000));
        assert_eq!(
            invalid.map(TankSnapshot::try_from).transpose(),
            Err(InvalidTank::MissingFluid)
        );
    }
}
