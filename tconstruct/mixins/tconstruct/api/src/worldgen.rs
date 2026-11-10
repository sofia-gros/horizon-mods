//! Host-independent rules from ExtraRootVariantPlacer and LeaveVineDecorator.
//!
//! The caller retains the source order of root variants and leaves and supplies
//! the feature's RNG, block/tag matching, provider sampling and world access.
//! The registered leaf-vine decorator uses the caller's editor and flags=19.
//! This core does not sample root providers, waterlog roots, run the parent
//! mangrove algorithm or prove native tree integration.

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

/// Bind the Java loop to the supplied editor. Live resolution fills the vine's
/// default properties before applying its one attachment property. A failed
/// explicit-flags write is an error; earlier vines are not rolled back.
struct EditorVines<'a> {
    world: &'a mut dyn pluto_lib::BlockEditor,
    vine: &'a pluto_lib::BlockState,
}

impl VineWorld for EditorVines<'_> {
    type Error = String;
    fn next_float(&mut self) -> f32 { self.world.random_float() }
    fn is_air(&mut self, pos: [i32; 3]) -> Result<bool, String> {
        self.world.block_state(pos).ok_or_else(|| format!("vine position unavailable: {pos:?}"))?;
        Ok(self.world.is_air(pos))
    }
    fn place_vine(&mut self, pos: [i32; 3], face: HorizontalDirection) -> Result<(), String> {
        let property = match face {
            HorizontalDirection::North => "north", HorizontalDirection::East => "east",
            HorizontalDirection::South => "south", HorizontalDirection::West => "west",
        };
        let state = self.vine.clone().with(property, "true");
        if self.world.set_block_state_with_flags(pos, &state, 19) { Ok(()) }
        else { Err(format!("vine write failed: {pos:?}")) }
    }
}

/// Register the original custom decorator, not an approximation of the native
/// fixed-vine decorator. Callers must supply the tree's ordered leaf list.
pub fn register() {
    use pluto_lib::{Identifier, BlockState, TreeDecorator, TreeDecoratorTypeRegistry};
    assert!(TreeDecoratorTypeRegistry::register(Identifier::parse("tconstruct:leave_vines").expect("fixed id"), |configuration| {
        let vine = configuration.get("vines").and_then(|v| v.as_str())
            .and_then(|text| Identifier::parse(text).ok())
            .ok_or_else(|| "vines must be a block identifier".to_owned())?;
        let probability = configuration.get("probability").and_then(|v| v.as_f64()).map(|p| p as f32)
            .filter(|p| p.is_finite() && (0.0..=1.0).contains(p))
            .ok_or_else(|| "probability must be in 0..=1".to_owned())?;
        let vine = BlockState { block: vine, properties: Default::default() };
        Ok(TreeDecorator::new(move |context| {
            decorate_vines(&mut EditorVines { world: context.world, vine: &vine }, context.leaves, probability)
        }))
    }), "tree decorator registration failed");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, VecDeque};

    #[test]
    fn registered_vines_validate_configuration_and_use_exact_flags_and_five_block_limit() {
        use pluto_lib::{BlockState, Identifier, MemoryBlockView, TreeDecoratorContext, TreeDecoratorTypeRegistry};
        register();
        let id = Identifier::parse("tconstruct:leave_vines").unwrap();
        for bad in [r#"{"vines":"minecraft:vine[north=true]","probability":1}"#, r#"{"vines":"minecraft:vine","probability":1.1}"#, r#"{"vines":"minecraft:vine"}"#] {
            assert!(TreeDecoratorTypeRegistry::decode(&id, &bad.parse().unwrap()).is_err());
        }
        // Java Codec.FLOAT converts to float before its range check.
        assert!(TreeDecoratorTypeRegistry::decode(&id, &r#"{"vines":"minecraft:vine","probability":1.000000001}"#.parse().unwrap()).is_ok());
        let configuration = r#"{"vines":"minecraft:vine","probability":1}"#.parse().unwrap();
        let decorator = TreeDecoratorTypeRegistry::decode(&id, &configuration).unwrap().unwrap();
        let mut world = MemoryBlockView::default();
        // Occupied north position: the next side (east) is the first write.
        world.blocks.insert([0, 10, -1], BlockState::parse("minecraft:stone").unwrap());
        decorator.decorate(&mut TreeDecoratorContext { world: &mut world, logs: &[], leaves: &[[0, 10, 0]], roots: &[] }).unwrap();
        assert_eq!(world.writes.len(), 15);
        assert_eq!(world.writes[0].0, [1, 10, 0]);
        assert_eq!(world.writes[0].1.property("west"), Some("true"));
        assert_eq!(world.writes[5].0, [0, 10, 1]);
        assert_eq!(world.writes[5].1.property("north"), Some("true"));
        assert_eq!(world.writes[10].0, [-1, 10, 0]);
        assert_eq!(world.writes[10].1.property("east"), Some("true"));
        assert!(world.writes.iter().all(|(_, state, flags)| *flags == 19 && state.properties.len() == 1));
        assert!(!world.blocks.contains_key(&[1, 5, 0]));
    }

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
