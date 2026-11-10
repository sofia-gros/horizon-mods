//! Host-independent rules from ExtraRootVariantPlacer and LeaveVineDecorator.
//!
//! The caller retains the source order of root variants and leaves and supplies
//! the feature's RNG, block/tag matching, provider sampling and world access.
//! This core does not sample root providers, waterlog roots, run the parent
//! mangrove algorithm, register features or prove native tree integration.

/// One ordered root predicate/provider pair. `Matches` may retain block IDs,
/// tags or a parsed holder set; `Provider` may retain the original provider JSON.
/// Neither is interpreted or reduced to a fixed block map here.
#[derive(Clone, Debug)]
pub struct RootVariant<Matches, Provider> {
    pub matches: Matches,
    pub provider: Provider,
}

/// ExtraRootVariantPlacer.placeRoot selects the first matching extra variant.
/// The callback tests the original position/state against each holder in order.
/// `None` delegates to the parent mangrove placer (including its first variant),
/// not to a made-up default provider. A match error stops before later variants.
pub fn select_root_variant<'a, Matches, Provider, Error>(
    variants: &'a [RootVariant<Matches, Provider>],
    mut matches: impl FnMut(&Matches) -> Result<bool, Error>,
) -> Result<Option<&'a Provider>, Error> {
    for variant in variants {
        if matches(&variant.matches)? {
            return Ok(Some(&variant.provider));
        }
    }
    Ok(None)
}

/// Horizontal directions/properties, not a host's numeric direction encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HorizontalDirection {
    North,
    East,
    South,
    West,
}

impl HorizontalDirection {
    fn relative(self, [x, y, z]: [i32; 3]) -> [i32; 3] {
        match self {
            Self::North => [x, y, z.wrapping_sub(1)],
            Self::East => [x.wrapping_add(1), y, z],
            Self::South => [x, y, z.wrapping_add(1)],
            Self::West => [x.wrapping_sub(1), y, z],
        }
    }

    fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::East => Self::West,
            Self::South => Self::North,
            Self::West => Self::East,
        }
    }
}

/// An adapter uses the supplied feature RNG, not a newly seeded/global RNG.
/// `place_vine` writes the configured vine's DEFAULT state with just the given
/// attachment property set true, as Java's placeVine does. It does not merge an
/// existing vine state. Read/write errors must be surfaced, not silently skipped.
pub trait VineWorld {
    type Error;
    fn next_float(&mut self) -> f32;
    fn is_air(&mut self, pos: [i32; 3]) -> Result<bool, Self::Error>;
    fn place_vine(&mut self, pos: [i32; 3], attached_face: HorizontalDirection) -> Result<(), Self::Error>;
}

/// Full LeaveVineDecorator.place/addHangingVine loop: ordered leaves, N/E/S/W,
/// one float per side, initial vine plus at most four downward vines. Coordinates
/// wrap like Java BlockPos offsets. The air query occurs BEFORE the remaining
/// length check, including the extra query below the fifth vine. No rollback is
/// attempted after an error; successful earlier writes remain in the world.
///
/// The source codec constrains normal probabilities to [0,1]. The comparison is
/// intentionally the direct Java `<`, preserving its behavior even for NaN.
pub fn decorate_vines<W: VineWorld>(
    world: &mut W,
    leaves: &[[i32; 3]],
    probability: f32,
) -> Result<(), W::Error> {
    use HorizontalDirection::{East, North, South, West};
    for &leaf in leaves {
        for direction in [North, East, South, West] {
            if world.next_float() < probability {
                let mut pos = direction.relative(leaf);
                if world.is_air(pos)? {
                    let face = direction.opposite();
                    world.place_vine(pos, face)?;
                    pos[1] = pos[1].wrapping_sub(1);
                    let mut remaining = 4;
                    // Keep the evaluation order of Java's &&. In particular, a
                    // failed sixth query propagates even though no sixth write
                    // would have been permitted by the length bound.
                    while world.is_air(pos)? && remaining > 0 {
                        world.place_vine(pos, face)?;
                        pos[1] = pos[1].wrapping_sub(1);
                        remaining -= 1;
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, VecDeque};

    #[test]
    fn root_variants_keep_order_provider_identity_and_parent_fallback() {
        let variants = [
            RootVariant { matches: "#example:early", provider: "weighted provider JSON" },
            RootVariant { matches: "example:block", provider: "state provider JSON" },
            RootVariant { matches: "#example:late", provider: "late provider JSON" },
        ];
        let mut calls = Vec::new();
        let selected = select_root_variant(&variants, |holder| {
            calls.push(*holder);
            Ok::<_, &'static str>(*holder != "#example:early")
        }).unwrap();
        assert!(std::ptr::eq(selected.unwrap(), &variants[1].provider));
        assert_eq!(calls, ["#example:early", "example:block"]);
        // Both predicates match: only the first is tested or selected.
        let selected = select_root_variant(&variants, |_| Ok::<_, ()>(true)).unwrap();
        assert!(std::ptr::eq(selected.unwrap(), &variants[0].provider));
        calls.clear();
        let fallback = select_root_variant(&variants, |holder| {
            calls.push(*holder);
            Ok::<_, ()>(false)
        }).unwrap();
        assert_eq!(fallback, None);
        assert_eq!(calls, ["#example:early", "example:block", "#example:late"]);
        let mut calls = 0;
        let error = select_root_variant(&variants, |_| {
            calls += 1;
            Err::<bool, _>("holder lookup failed")
        });
        assert_eq!(error, Err("holder lookup failed"));
        assert_eq!(calls, 1);
        let empty: [RootVariant<&str, &str>; 0] = [];
        assert_eq!(select_root_variant(&empty, |_| -> Result<bool, ()> { panic!("empty list") }), Ok(None));
    }

    #[derive(Debug, PartialEq)]
    enum Event {
        Random(u32),
        Air([i32; 3]),
        Write([i32; 3], HorizontalDirection),
    }

    struct World {
        floats: VecDeque<f32>,
        air: BTreeSet<[i32; 3]>,
        trace: Vec<Event>,
        fail_read: Option<[i32; 3]>,
        fail_write: Option<[i32; 3]>,
    }

    impl VineWorld for World {
        type Error = &'static str;
        fn next_float(&mut self) -> f32 {
            let value = self.floats.pop_front().expect("unexpected RNG consumption");
            self.trace.push(Event::Random(value.to_bits()));
            value
        }
        fn is_air(&mut self, pos: [i32; 3]) -> Result<bool, Self::Error> {
            self.trace.push(Event::Air(pos));
            if self.fail_read == Some(pos) { return Err("read failed"); }
            Ok(self.air.contains(&pos))
        }
        fn place_vine(&mut self, pos: [i32; 3], face: HorizontalDirection) -> Result<(), Self::Error> {
            self.trace.push(Event::Write(pos, face));
            if self.fail_write == Some(pos) { return Err("write failed"); }
            self.air.remove(&pos);
            Ok(())
        }
    }

    #[test]
    fn vines_preserve_java_trace_bounds_wrapping_nan_and_immediate_errors() {
        use Event::{Air, Random, Write};
        use HorizontalDirection::South;
        let (lo, hi) = (i32::MIN, i32::MAX);
        // North wraps z; downward growth wraps y. Six air positions permit five
        // writes and the required final air query, but never a sixth write.
        let column = [[hi, lo, hi], [hi, hi, hi], [hi, hi - 1, hi],
                      [hi, hi - 2, hi], [hi, hi - 3, hi], [hi, hi - 4, hi]];
        let mut world = World {
            floats: [0.1, 0.2, f32::NAN, 0.9, 0.1, 0.1, 0.1, 0.1].into(),
            air: column.into_iter().collect(), trace: Vec::new(),
            fail_read: None, fail_write: None,
        };
        decorate_vines(&mut world, &[[hi, lo, lo], [0, 0, 0]], 0.5).unwrap();
        assert_eq!(world.trace, [
            Random(0.1f32.to_bits()), Air(column[0]), Write(column[0], South),
            Air(column[1]), Write(column[1], South),
            Air(column[2]), Write(column[2], South),
            Air(column[3]), Write(column[3], South),
            Air(column[4]), Write(column[4], South), Air(column[5]),
            Random(0.2f32.to_bits()), Air([lo, lo, lo]),
            Random(f32::NAN.to_bits()), Random(0.9f32.to_bits()),
            Random(0.1f32.to_bits()), Air([0, 0, -1]),
            Random(0.1f32.to_bits()), Air([1, 0, 0]),
            Random(0.1f32.to_bits()), Air([0, 0, 1]),
            Random(0.1f32.to_bits()), Air([-1, 0, 0]),
        ]);
        assert_eq!(world.air, [column[5]].into_iter().collect());
        assert!(world.floats.is_empty());
        // A query error below vine five still propagates: isAir is evaluated
        // before i>0, not optimized away. No later side consumes a random float.
        let mut failed = World {
            floats: [0.1, 0.8].into(), air: column.into_iter().collect(),
            trace: Vec::new(), fail_read: Some(column[5]), fail_write: None,
        };
        assert_eq!(decorate_vines(&mut failed, &[[hi, lo, lo]], 0.5), Err("read failed"));
        assert_eq!(failed.trace.last(), Some(&Air(column[5])));
        assert_eq!(failed.trace.iter().filter(|event| matches!(event, Write(..))).count(), 5);
        assert_eq!(failed.floats, VecDeque::from([0.8]));
        failed.trace.clear();
        failed.floats = [0.1, 0.8].into();
        failed.air = column.into_iter().collect();
        failed.fail_read = None;
        failed.fail_write = Some(column[0]);
        assert_eq!(decorate_vines(&mut failed, &[[hi, lo, lo]], 0.5), Err("write failed"));
        assert_eq!(failed.trace, [Random(0.1f32.to_bits()), Air(column[0]), Write(column[0], South)]);
        assert_eq!(failed.floats, VecDeque::from([0.8]));
        // An invalid NaN probability must not turn every side into a placement.
        failed.trace.clear();
        failed.floats = [0.0; 4].into();
        failed.fail_write = None;
        decorate_vines(&mut failed, &[[0, 0, 0]], f32::NAN).unwrap();
        assert_eq!(failed.trace, [Random(0.0f32.to_bits()), Random(0.0f32.to_bits()),
                                  Random(0.0f32.to_bits()), Random(0.0f32.to_bits())]);
    }
}
