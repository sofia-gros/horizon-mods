//! Finding a heating structure (`MultiblockCuboid`): from the controller, the walls of a box of air are found, then its layers up and
//! down, then the floor and the ceiling. The world is asked only four things, in [`Level`].

/// A block position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Pos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
    fn offset(self, x: i32, y: i32, z: i32) -> Self {
        Self::new(self.x + x, self.y + y, self.z + z)
    }
    fn above(self, n: i32) -> Self {
        self.offset(0, n, 0)
    }
}

/// The horizontal directions, in the order of Minecraft's `get2DDataValue`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Facing {
    South = 0,
    West = 1,
    North = 2,
    East = 3,
}

impl Facing {
    const ALL: [Facing; 4] = [Facing::South, Facing::West, Facing::North, Facing::East];

    fn step(self) -> (i32, i32) {
        match self {
            Facing::South => (0, 1),
            Facing::West => (-1, 0),
            Facing::North => (0, -1),
            Facing::East => (1, 0),
        }
    }

    fn opposite(self) -> Facing {
        Facing::ALL[(self as usize + 2) % 4]
    }
}

/// Which part of the box a block belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Floor,
    Ceiling,
    Wall,
}

/// What the world answers.
pub trait Level {
    /// Whether the chunk of `pos` is loaded.
    fn is_loaded(&self, pos: Pos) -> bool;
    /// Whether every chunk between the two positions is loaded.
    fn has_chunks(&self, from: Pos, to: Pos) -> bool;
    /// Air, or a block of the structure-air tag: the space inside.
    fn is_inner(&self, pos: Pos) -> bool;
    /// Whether the block is air proper (a structure-air block is inner but not air: it is part of the structure).
    fn is_air(&self, pos: Pos) -> bool;
    /// Whether the block may be a wall, floor or ceiling block (or, with `frame`, a frame block) of the structure.
    fn is_valid(&self, pos: Pos, side: Side, frame: bool) -> bool;
}

/// Why a structure was not found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    NoAttempt,
    NotLoaded,
    InvalidInnerBlock,
    TooLarge { x: i32, z: i32, limit: i32 },
    InvalidFloorBlock,
    InvalidCeilingBlock,
    InvalidWallBlock,
    InvalidFloorFrame,
    InvalidCeilingFrame,
    InvalidWallFrame,
}

/// A failed detection: what was wrong and where, when there is a place to point at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Failure {
    pub pos: Option<Pos>,
    pub problem: Problem,
}

fn fail(pos: Option<Pos>, problem: Problem) -> Failure {
    Failure { pos, problem }
}

/// A found structure: its two corners (the outer blocks) and the blocks inside that are not air.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Structure {
    pub min: Pos,
    pub max: Pos,
    pub extra: Vec<Pos>,
}

/// The shape a kind of structure has.
#[derive(Clone, Copy, Debug)]
pub struct Cuboid {
    pub has_floor: bool,
    pub has_frame: bool,
    pub has_ceiling: bool,
    pub max_height: i32,
    pub inner_limit: i32,
}

impl Cuboid {
    pub fn new(has_floor: bool, has_frame: bool, has_ceiling: bool) -> Self {
        Self { has_floor, has_frame, has_ceiling, max_height: 64, inner_limit: 14 }
    }

    fn outer_pos(&self, level: &dyn Level, mut pos: Pos, facing: Facing, limit: i32) -> Pos {
        let (dx, dz) = facing.step();
        let mut i = 0;
        while i < limit && level.is_loaded(pos) && level.is_inner(pos) {
            pos = pos.offset(dx, 0, dz);
            i += 1;
        }
        pos
    }

    /// `detectMultiblock`: the structure around the block behind the controller at `master`, which faces `facing`.
    pub fn detect(&self, level: &dyn Level, master: Pos, facing: Facing) -> Result<Structure, Failure> {
        let mut extra = Vec::new();
        let (bx, bz) = facing.opposite().step();
        let mut center = master.offset(bx, 0, bz);
        let mut needed_cap = None;
        if !level.is_inner(center) {
            if !self.has_frame || (!self.has_floor && !self.has_ceiling) {
                return Err(fail(Some(center), Problem::InvalidInnerBlock));
            }
            // the controller may sit in the floor or the ceiling layer: pick one cap, the floor when there is one
            if self.has_floor {
                needed_cap = Some(Side::Floor);
                center = center.above(1);
            } else {
                needed_cap = Some(Side::Ceiling);
                center = center.above(-1);
            }
            if !level.is_inner(center) {
                return Err(fail(Some(center), Problem::InvalidInnerBlock));
            }
        }

        let mut edges = [0i32; 4];
        for facing in Facing::ALL {
            let pos = self.outer_pos(level, center, facing, self.inner_limit + 1);
            edges[facing as usize] = (pos.x - center.x) + (pos.z - center.z);
        }
        let (s, w, n, e) = (edges[Facing::South as usize], edges[Facing::West as usize], edges[Facing::North as usize], edges[Facing::East as usize]);
        let xd = (s - n) - 1;
        let zd = (e - w) - 1;
        if xd > self.inner_limit || zd > self.inner_limit {
            return Err(fail(None, Problem::TooLarge { x: xd, z: zd, limit: self.inner_limit }));
        }
        let from = center.offset(w, 0, n);
        let to = center.offset(e, 0, s);

        // the layer the controller is in
        self.detect_layer(level, from, to, &mut extra)?;

        // downwards, unless the controller is in the floor
        let mut min_layer = -1;
        let mut remaining = self.max_height - 1;
        let mut layer_result: Result<(), Failure> = Ok(());
        if needed_cap != Some(Side::Floor) {
            while min_layer > -remaining {
                layer_result = self.detect_layer(level, from.above(min_layer), to.above(min_layer), &mut extra);
                if layer_result.is_err() {
                    break;
                }
                min_layer -= 1;
            }
        }
        remaining += min_layer + 1;
        if self.has_floor {
            self.detect_cap(level, from.above(min_layer), to.above(min_layer), Side::Floor)?;
        } else {
            min_layer += 1;
        }

        // upwards, unless the controller is in the ceiling
        let mut max_layer = 1;
        if needed_cap != Some(Side::Ceiling) {
            while max_layer < remaining {
                layer_result = self.detect_layer(level, from.above(max_layer), to.above(max_layer), &mut extra);
                if layer_result.is_err() {
                    break;
                }
                max_layer += 1;
            }
        }
        if self.has_ceiling {
            self.detect_cap(level, from.above(max_layer), to.above(max_layer), Side::Ceiling)?;
        } else {
            max_layer -= 1;
        }
        let _ = layer_result;
        extra.sort();
        extra.dedup();
        Ok(Structure { min: from.above(min_layer), max: to.above(max_layer), extra })
    }

    fn detect_cap(&self, level: &dyn Level, from: Pos, to: Pos, side: Side) -> Result<(), Failure> {
        if !level.has_chunks(from, to) {
            return Err(fail(None, Problem::NotLoaded));
        }
        let y = from.y;
        if self.has_frame {
            let frame_error = if side == Side::Ceiling { Problem::InvalidCeilingFrame } else { Problem::InvalidFloorFrame };
            let check = |pos: Pos| if level.is_valid(pos, side, true) { Ok(()) } else { Err(fail(Some(pos), frame_error)) };
            for x in from.x..=to.x {
                check(Pos::new(x, y, from.z))?;
                check(Pos::new(x, y, to.z))?;
            }
            for z in from.z + 1..to.z {
                check(Pos::new(from.x, y, z))?;
                check(Pos::new(to.x, y, z))?;
            }
        }
        let block_error = if side == Side::Ceiling { Problem::InvalidCeilingBlock } else { Problem::InvalidFloorBlock };
        for z in from.z + 1..to.z {
            for x in from.x + 1..to.x {
                let pos = Pos::new(x, y, z);
                if !level.is_valid(pos, side, false) {
                    return Err(fail(Some(pos), block_error));
                }
            }
        }
        Ok(())
    }

    fn detect_layer(&self, level: &dyn Level, from: Pos, to: Pos, extra: &mut Vec<Pos>) -> Result<(), Failure> {
        if !level.has_chunks(from, to) {
            return Err(fail(None, Problem::NotLoaded));
        }
        let y = from.y;
        let mut candidates = Vec::new();
        for x in from.x + 1..to.x {
            for z in from.z + 1..to.z {
                let pos = Pos::new(x, y, z);
                if !level.is_inner(pos) {
                    return Err(fail(Some(pos), Problem::InvalidInnerBlock));
                }
                if !level.is_air(pos) {
                    candidates.push(pos);
                }
            }
        }
        if self.has_frame {
            for pos in [from, Pos::new(from.x, y, to.z), Pos::new(to.x, y, from.z), to] {
                if !level.is_valid(pos, Side::Wall, true) {
                    return Err(fail(Some(pos), Problem::InvalidWallFrame));
                }
            }
        }
        for x in from.x + 1..to.x {
            for pos in [Pos::new(x, y, from.z), Pos::new(x, y, to.z)] {
                if !level.is_valid(pos, Side::Wall, false) {
                    return Err(fail(Some(pos), Problem::InvalidWallBlock));
                }
            }
        }
        for z in from.z + 1..to.z {
            for pos in [Pos::new(from.x, y, z), Pos::new(to.x, y, z)] {
                if !level.is_valid(pos, Side::Wall, false) {
                    return Err(fail(Some(pos), Problem::InvalidWallBlock));
                }
            }
        }
        extra.extend(candidates);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Clone, Copy, PartialEq)]
    enum B {
        Air,
        StructureAir,
        Wall,
        Frame,
        Stone,
    }

    /// A world of listed blocks; everything else is stone.
    struct World {
        blocks: HashMap<Pos, B>,
    }
    impl World {
        fn get(&self, pos: Pos) -> B {
            self.blocks.get(&pos).copied().unwrap_or(B::Stone)
        }
        /// A melter-like box: `w` x `d` inside, `h` layers high, walls, a floor and no ceiling, the controller in the south wall at mid height.
        fn box_of(w: i32, d: i32, h: i32) -> Self {
            let mut blocks = HashMap::new();
            for y in 0..=h {
                for x in -1..=w {
                    for z in -1..=d {
                        let edge = x == -1 || x == w || z == -1 || z == d;
                        let b = if y == 0 { B::Wall } else if edge { B::Wall } else { B::Air };
                        blocks.insert(Pos::new(x, y, z), b);
                    }
                }
            }
            Self { blocks }
        }
    }
    impl Level for World {
        fn is_loaded(&self, _: Pos) -> bool {
            true
        }
        fn has_chunks(&self, _: Pos, _: Pos) -> bool {
            true
        }
        fn is_inner(&self, pos: Pos) -> bool {
            matches!(self.get(pos), B::Air | B::StructureAir)
        }
        fn is_air(&self, pos: Pos) -> bool {
            self.get(pos) == B::Air
        }
        fn is_valid(&self, pos: Pos, _: Side, frame: bool) -> bool {
            match self.get(pos) {
                B::Wall => !frame,
                B::Frame => true,
                _ => false,
            }
        }
    }

    fn smeltery() -> Cuboid {
        Cuboid { has_floor: true, has_frame: false, has_ceiling: false, max_height: 64, inner_limit: 14 }
    }

    #[test]
    fn an_open_box_with_a_floor_is_found_layer_by_layer_up_to_the_first_non_inner_layer() {
        let mut world = World::box_of(3, 3, 4);
        // the controller replaces a south wall block (z = 3) at height 2, facing south, so the center is behind it
        let master = Pos::new(1, 2, 3);
        world.blocks.insert(master, B::Wall);
        let found = smeltery().detect(&world, master, Facing::South).unwrap();
        assert_eq!((found.min, found.max), (Pos::new(-1, 0, -1), Pos::new(3, 4, 3)));
        assert!(found.extra.is_empty(), "only air inside");
    }

    #[test]
    fn a_block_inside_that_is_not_air_is_listed_and_a_foreign_one_stops_the_search() {
        let mut world = World::box_of(3, 3, 3);
        world.blocks.insert(Pos::new(1, 1, 1), B::StructureAir);
        let master = Pos::new(1, 2, 3);
        world.blocks.insert(master, B::Wall);
        let found = smeltery().detect(&world, master, Facing::South).unwrap();
        assert_eq!(found.extra, vec![Pos::new(1, 1, 1)]);
        world.blocks.insert(Pos::new(0, 2, 0), B::Stone); // in the controller's own layer; in the lowest layer it would read as a bad floor, as in the original
        let error = smeltery().detect(&world, master, Facing::South).unwrap_err();
        assert_eq!((error.pos, error.problem), (Some(Pos::new(0, 2, 0)), Problem::InvalidInnerBlock));
    }

    #[test]
    fn a_hole_in_a_wall_a_missing_floor_and_a_box_too_big_are_reported() {
        let master = Pos::new(1, 2, 3);
        let mut hole = World::box_of(3, 3, 3);
        hole.blocks.insert(master, B::Wall);
        hole.blocks.insert(Pos::new(-1, 2, 1), B::Stone);
        assert_eq!(smeltery().detect(&hole, master, Facing::South).unwrap_err().problem, Problem::InvalidWallBlock);

        let mut no_floor = World::box_of(3, 3, 3);
        no_floor.blocks.insert(master, B::Wall);
        no_floor.blocks.insert(Pos::new(1, 0, 1), B::Stone);
        let error = smeltery().detect(&no_floor, master, Facing::South).unwrap_err();
        assert_eq!((error.pos, error.problem), (Some(Pos::new(1, 0, 1)), Problem::InvalidFloorBlock));

        let mut big = World::box_of(20, 3, 3);
        big.blocks.insert(Pos::new(1, 2, 3), B::Wall);
        assert!(matches!(smeltery().detect(&big, Pos::new(1, 2, 3), Facing::South).unwrap_err().problem, Problem::TooLarge { .. }));
    }

    /// A foundry-like world: a frame of `Frame` blocks on the edges of the floor, ceiling and corners, walls between, floor and ceiling
    /// plates, `h` layers of air inside a `w` x `d` box.
    fn framed(w: i32, d: i32, h: i32) -> World {
        let mut blocks = HashMap::new();
        for y in 0..=h + 1 {
            for x in -1..=w {
                for z in -1..=d {
                    let on_x_edge = x == -1 || x == w;
                    let on_z_edge = z == -1 || z == d;
                    let cap = y == 0 || y == h + 1;
                    let b = if cap {
                        if on_x_edge || on_z_edge { B::Frame } else { B::Wall }
                    } else if on_x_edge && on_z_edge {
                        B::Frame
                    } else if on_x_edge || on_z_edge {
                        B::Wall
                    } else {
                        B::Air
                    };
                    blocks.insert(Pos::new(x, y, z), b);
                }
            }
        }
        World { blocks }
    }

    fn foundry() -> Cuboid {
        Cuboid { has_floor: true, has_frame: true, has_ceiling: true, max_height: 64, inner_limit: 14 }
    }

    #[test]
    fn a_framed_box_with_a_floor_and_a_ceiling_is_found_between_its_caps() {
        let mut world = framed(3, 3, 3);
        let master = Pos::new(1, 2, 3);
        world.blocks.insert(master, B::Wall);
        let found = foundry().detect(&world, master, Facing::South).unwrap();
        assert_eq!((found.min, found.max), (Pos::new(-1, 0, -1), Pos::new(3, 4, 3)));
    }

    #[test]
    fn a_missing_frame_block_or_cap_plate_is_reported_where_it_is() {
        let master = Pos::new(1, 2, 3);
        let mut no_corner = framed(3, 3, 3);
        no_corner.blocks.insert(master, B::Wall);
        no_corner.blocks.insert(Pos::new(-1, 2, -1), B::Wall); // a wall block where the corner post should be
        let error = foundry().detect(&no_corner, master, Facing::South).unwrap_err();
        assert_eq!((error.pos, error.problem), (Some(Pos::new(-1, 2, -1)), Problem::InvalidWallFrame));

        let mut no_ceiling = framed(3, 3, 3);
        no_ceiling.blocks.insert(master, B::Wall);
        no_ceiling.blocks.insert(Pos::new(1, 4, 1), B::Stone);
        let error = foundry().detect(&no_ceiling, master, Facing::South).unwrap_err();
        assert_eq!((error.pos, error.problem), (Some(Pos::new(1, 4, 1)), Problem::InvalidCeilingBlock));

        let mut no_floor_frame = framed(3, 3, 3);
        no_floor_frame.blocks.insert(master, B::Wall);
        no_floor_frame.blocks.insert(Pos::new(1, 0, -1), B::Stone);
        let error = foundry().detect(&no_floor_frame, master, Facing::South).unwrap_err();
        assert_eq!((error.pos, error.problem), (Some(Pos::new(1, 0, -1)), Problem::InvalidFloorFrame));
    }

    #[test]
    fn a_controller_not_in_front_of_air_cannot_start_a_search() {
        let world = World::box_of(3, 3, 3);
        let error = smeltery().detect(&world, Pos::new(1, 2, -1), Facing::South).unwrap_err();
        assert_eq!(error.problem, Problem::InvalidInnerBlock);
        assert_eq!(smeltery().detect(&world, Pos::new(1, 2, 3), Facing::North).unwrap_err().pos, Some(Pos::new(1, 2, 4)));
    }
}
