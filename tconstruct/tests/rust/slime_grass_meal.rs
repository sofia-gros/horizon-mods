#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct BlockPos {
    x: i32,
    y: i32,
    z: i32,
}
impl BlockPos {
    fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}
#[path = "../../mixins/tconstruct/source-slime-grass-meal-algorithm.rs"]
mod algorithm;
use algorithm::{grow, GrowWorld};
use std::collections::HashSet;
struct World {
    plants: HashSet<BlockPos>,
    fern: usize,
    tall: usize,
    rolls: Vec<i32>,
    full: bool,
    valid: bool,
    bounds: Vec<i32>,
}
impl GrowWorld for World {
    fn roll(&mut self, bound: i32) -> Option<i32> {
        self.bounds.push(bound);
        Some(self.rolls.pop().unwrap_or(1) % bound)
    }
    fn full(&self, _: BlockPos) -> bool {
        self.full
    }
    fn valid_base(&self, _: BlockPos) -> bool {
        self.valid
    }
    fn air(&self, pos: BlockPos) -> bool {
        !self.plants.contains(&pos)
    }
    fn place(&mut self, pos: BlockPos, fern: bool) {
        if self.plants.insert(pos) {
            if fern {
                self.fern += 1
            } else {
                self.tall += 1
            }
        }
    }
}
fn world() -> World {
    World {
        plants: HashSet::new(),
        fern: 0,
        tall: 0,
        rolls: Vec::new(),
        full: false,
        valid: true,
        bounds: Vec::new(),
    }
}
#[test]
fn initial_plants_use_fern_and_existing_plant_is_not_overwritten() {
    let mut w = world();
    w.rolls.push(0);
    grow(&mut w, BlockPos::new(0, 0, 0));
    assert_eq!(w.fern, 1);
    assert_eq!(w.tall, 0);
    assert_eq!(w.plants.len(), 1);
    assert_eq!(w.bounds[0], 32);
}
#[test]
fn initial_high_choice_yields_tall_grass() {
    let mut w = world();
    w.rolls.push(31);
    grow(&mut w, BlockPos::new(0, 0, 0));
    assert_eq!((w.fern, w.tall), (0, 1));
}
#[test]
fn invalid_or_opaque_walks_stop_without_placing() {
    for full in [false, true] {
        let mut w = world();
        w.valid = false;
        w.full = full;
        grow(&mut w, BlockPos::new(0, 0, 0));
        assert_eq!(w.plants.len(), 1);
        assert_eq!(w.bounds.iter().filter(|&&b| b == 32).count(), 1);
        assert_eq!(w.bounds.iter().filter(|&&b| b == 3).count(), 112 * 4);
    }
}
#[test]
fn overflow_never_writes_or_draws_random() {
    let mut w = world();
    grow(&mut w, BlockPos::new(0, i32::MAX, 0));
    assert!(w.bounds.is_empty());
    assert!(w.plants.is_empty());
}

#[test]
fn walk_offsets_and_source_random_order_place_on_new_grass() {
    let mut w = world();
    // First choose tall grass at the origin, then after 16 attempts walk
    // east, keep the height, and choose a fern.
    w.rolls = vec![0, 1, 0, 1, 2, 31];
    grow(&mut w, BlockPos::new(0, 0, 0));
    assert!(w.plants.contains(&BlockPos::new(0, 1, 0)));
    assert!(w.plants.contains(&BlockPos::new(1, 1, 0)));
    assert_eq!((w.fern, w.tall), (1, 1));
    assert_eq!(&w.bounds[..6], &[32, 3, 3, 3, 3, 32]);
}

#[test]
fn plant_selection_boundary_is_five_vs_six() {
    for (choice, expected) in [(5, (1, 0)), (6, (0, 1))] {
        let mut w = world();
        w.rolls.push(choice);
        grow(&mut w, BlockPos::new(0, 0, 0));
        assert_eq!((w.fern, w.tall), expected);
    }
}
