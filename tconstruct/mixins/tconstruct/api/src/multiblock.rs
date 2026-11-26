//! Detecting the smeltery and foundry structure, from SlimeKnights MultiblockCuboid, MultiblockStructureData,
//! HeatingStructureMultiblock, SmelteryMultiblock, FoundryMultiblock and MultiblockResult (MIT; see LICENSE): find the
//! cuboid around the controller, layer by layer, and check the floor, the walls and, for the foundry, the frame.
//! The world is a lookup closure (`Lookup`): the block at a position, `None` where its chunk is not loaded. The host
//! side builds it from PlutoLib's world reads and block tags; nothing here knows the game.
use pluto_lib::{Direction, Nbt, NbtCompound, NbtCompoundExt};
use std::collections::BTreeSet;

pub(crate) type Pos = [i32; 3];

fn offset(pos: Pos, dx: i32, dy: i32, dz: i32) -> Pos { [pos[0] + dx, pos[1] + dy, pos[2] + dz] }

/// The four horizontal directions in `Direction.Plane.HORIZONTAL` order.
const HORIZONTAL: [Direction; 4] = [Direction::North, Direction::East, Direction::South, Direction::West];

fn step(direction: Direction) -> Pos {
    match direction {
        Direction::North => [0, 0, -1],
        Direction::South => [0, 0, 1],
        Direction::West => [-1, 0, 0],
        Direction::East => [1, 0, 0],
        Direction::Up => [0, 1, 0],
        Direction::Down => [0, -1, 0],
    }
}

fn opposite(direction: Direction) -> Direction {
    match direction {
        Direction::North => Direction::South,
        Direction::South => Direction::North,
        Direction::West => Direction::East,
        Direction::East => Direction::West,
        Direction::Up => Direction::Down,
        Direction::Down => Direction::Up,
    }
}

/// Which face of the cube a layer is part of (`CuboidSide`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side { Floor, Ceiling, Wall }

/// What detection needs to know of one block, already classified for the structure being detected: the tags `smeltery` and
/// `smeltery/floor`, `smeltery/tanks`, `smeltery/wall` (or the foundry's), `structure_air`, and the `in_structure` property.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Cell {
    /// Air or in the `structure_air` tag (`isAirBlock`).
    pub air: bool,
    /// The `in_structure` block state property; `None` for a block that has none, which is never part of a structure.
    pub in_structure: Option<bool>,
    /// A servant block entity that already answers to another controller (`IServantLogic.isValidMaster` is false).
    pub foreign: bool,
    /// In the structure's block tag at all (`isValidBlock(Block)`).
    pub any: bool,
    pub floor: bool,
    pub tank: bool,
    pub wall: bool,
}

/// The world: the block at a position, `None` where its chunk is not loaded.
pub(crate) type Lookup<'a> = &'a dyn Fn(Pos) -> Option<Cell>;

/// MultiblockResult: whether the structure formed, and if not which block is to blame and why (a translation key and its arguments).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MultiblockResult { pub success: bool, pub pos: Option<Pos>, pub key: &'static str, pub args: Vec<i32> }

impl MultiblockResult {
    pub(crate) fn success() -> Self { Self { success: true, pos: None, key: "", args: Vec::new() } }
    fn error(pos: Option<Pos>, key: &'static str) -> Self { Self { success: false, pos, key, args: Vec::new() } }
}

pub(crate) const NO_ATTEMPT: &str = "multiblock.tconstruct.generic.no_attempt";
pub(crate) const NOT_LOADED: &str = "multiblock.tconstruct.generic.not_loaded";
pub(crate) const TOO_LARGE: &str = "multiblock.tconstruct.generic.too_large";
pub(crate) const INVALID_INNER_BLOCK: &str = "multiblock.tconstruct.generic.invalid_inner_block";
pub(crate) const INVALID_FLOOR_BLOCK: &str = "multiblock.tconstruct.generic.invalid_floor_block";
/// The original's INVALID_CEILING_BLOCK names the floor key too.
pub(crate) const INVALID_CEILING_BLOCK: &str = "multiblock.tconstruct.generic.invalid_floor_block";
pub(crate) const INVALID_WALL_BLOCK: &str = "multiblock.tconstruct.generic.invalid_wall_block";
pub(crate) const INVALID_FLOOR_FRAME: &str = "multiblock.tconstruct.generic.invalid_floor_frame";
pub(crate) const INVALID_CEILING_FRAME: &str = "multiblock.tconstruct.generic.invalid_ceiling_frame";
pub(crate) const INVALID_WALL_FRAME: &str = "multiblock.tconstruct.generic.invalid_wall_frame";

const TAG_EXTRA_POS: &str = "extra";
const TAG_MIN: &str = "min";
const TAG_MAX: &str = "max";
const TAG_TANKS: &str = "tanks";
const TAG_INSIDE_CHECK: &str = "insideCheck";

pub(crate) fn within(pos: Pos, min: Pos, max: Pos) -> bool { (0..3).all(|axis| pos[axis] >= min[axis] && pos[axis] <= max[axis]) }

/// MultiblockStructureData plus HeatingStructureMultiblock.StructureData: the size and contents of a formed structure.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StructureData {
    pub min: Pos,
    pub max: Pos,
    /// Blocks not in a standard position: whatever is inside that is not air.
    pub extra: BTreeSet<Pos>,
    has_floor: bool,
    has_frame: bool,
    has_ceiling: bool,
    pub min_inside: Pos,
    pub max_inside: Pos,
    pub inner_x: i32,
    pub inner_y: i32,
    pub inner_z: i32,
    /// The area inside the walls as a box (`getBounds`): min corner, then max corner plus one.
    pub bounds: [f64; 6],
    /// Positions of every tank in the structure area, in the order detection met them.
    pub tanks: Vec<Pos>,
    /// The next position the inside check looks at.
    inside_check: Option<Pos>,
}

impl StructureData {
    fn new(min: Pos, max: Pos, extra: BTreeSet<Pos>, has_floor: bool, has_frame: bool, has_ceiling: bool, tanks: Vec<Pos>) -> Self {
        let min_inside = offset(min, 1, i32::from(has_floor), 1);
        let max_inside = offset(max, -1, if has_ceiling { -1 } else { 0 }, -1);
        let upper = offset(max_inside, 1, 1, 1);
        Self {
            min, max, extra, has_floor, has_frame, has_ceiling, min_inside, max_inside,
            inner_x: max_inside[0] - min_inside[0] + 1,
            inner_y: max_inside[1] - min_inside[1] + 1,
            inner_z: max_inside[2] - min_inside[2] + 1,
            bounds: [min_inside[0] as f64, min_inside[1] as f64, min_inside[2] as f64, upper[0] as f64, upper[1] as f64, upper[2] as f64],
            tanks, inside_check: None,
        }
    }

    pub(crate) fn within_bounds(&self, pos: Pos) -> bool { within(pos, self.min, self.max) }
    pub(crate) fn is_inside(&self, pos: Pos) -> bool { within(pos, self.min_inside, self.max_inside) }
    pub(crate) fn contains(&self, pos: Pos) -> bool { self.within_bounds(pos) && self.contains_base(pos) }

    fn contains_base(&self, pos: Pos) -> bool {
        // blocks in the inner region are added to the extra positions, fall back to that
        if !self.is_inside(pos) {
            // with a frame the shape is a full cube
            if self.has_frame { return true; }
            // otherwise a frame block is any block touching two edges
            let mut edges = 0;
            if pos[0] == self.min[0] || pos[0] == self.max[0] { edges += 1; }
            if pos[2] == self.min[2] || pos[2] == self.max[2] { edges += 1; }
            // the original compares x with the maximum for the ceiling, as written
            if (self.has_floor && pos[1] == self.min[1]) || (self.has_ceiling && pos[0] == self.max[0]) { edges += 1; }
            if edges < 2 { return true; }
        }
        self.extra.contains(&pos)
    }

    /// Whether the position is exactly one block above the structure.
    pub(crate) fn is_directly_above(&self, pos: Pos) -> bool {
        pos[0] >= self.min[0] && pos[2] >= self.min[2] && pos[0] <= self.max[0] && pos[2] <= self.max[2] && pos[1] == self.max[1] + 1
    }

    /// `forEachContained`, in x, y, z order.
    pub(crate) fn contained(&self) -> Vec<Pos> {
        let mut result = Vec::new();
        for x in self.min[0]..=self.max[0] {
            for y in self.min[1]..=self.max[1] {
                for z in self.min[2]..=self.max[2] {
                    if self.contains_base([x, y, z]) { result.push([x, y, z]); }
                }
            }
        }
        result
    }

    /// `assignMaster`: blocks new to the structure learn their master, blocks only in the old one forget it.
    /// `mark(pos, true)` joins a block to the structure, `mark(pos, false)` frees it.
    pub(crate) fn assign_master(&self, old: Option<&StructureData>, mark: &mut dyn FnMut(Pos, bool)) {
        for pos in self.contained() {
            if old.is_none_or(|old| !old.contains(pos)) { mark(pos, true); }
        }
        if let Some(old) = old {
            for pos in old.contained() {
                if !self.contains(pos) { mark(pos, false); }
            }
        }
    }

    /// `clearMaster`.
    pub(crate) fn clear_master(&self, mark: &mut dyn FnMut(Pos, bool)) {
        for pos in self.contained() { mark(pos, false); }
    }

    pub(crate) fn has_tanks(&self) -> bool { !self.tanks.is_empty() }

    /// The next inside position to check (`getNextInsideCheck`): a walk over the whole inside, row by row, layer by layer.
    pub(crate) fn next_inside_check(&mut self) -> Pos {
        let next = self.after_inside(self.inside_check);
        self.inside_check = Some(next);
        next
    }

    fn after_inside(&self, prev: Option<Pos>) -> Pos {
        let (min, max) = (self.min_inside, self.max_inside);
        let Some(prev) = prev else { return min };
        // smaller than min means the structure size changed
        if prev[0] < min[0] || prev[1] < min[1] || prev[2] < min[2] { return min; }
        if prev[2] >= max[2] {
            if prev[0] >= max[0] {
                if prev[1] >= max[1] { min } else { [min[0], prev[1] + 1, min[2]] }
            } else {
                [prev[0] + 1, prev[1], min[2]]
            }
        } else {
            offset(prev, 0, 0, 1)
        }
    }

    /// The number of blocks making up the walls and floor, as the structure counts them.
    pub(crate) fn perimeter_count(&self) -> i32 {
        let dx = self.max_inside[0] - self.min_inside[0];
        let dy = self.max_inside[1] - self.min_inside[1];
        let dz = self.max_inside[2] - self.min_inside[2];
        2 * (dx * dy) + 2 * (dy * dz) + dx * dz
    }

    /// The client's copy: bounds and tanks, none of the extra positions.
    pub(crate) fn client(&self) -> StructureData {
        StructureData::new(self.min, self.max, BTreeSet::new(), self.has_floor, self.has_frame, self.has_ceiling, self.tanks.clone())
    }
}

fn write_pos(pos: Pos) -> Nbt {
    let mut tag = NbtCompound::new();
    for (key, value) in ["X", "Y", "Z"].into_iter().zip(pos) { tag.insert(key.to_owned(), Nbt::Int(value)); }
    Nbt::Compound(tag)
}

fn read_pos(tag: &NbtCompound) -> Pos { [tag.get_int("X", 0) as i32, tag.get_int("Y", 0) as i32, tag.get_int("Z", 0) as i32] }

fn sub(pos: Pos, base: Pos) -> Pos { offset(pos, -base[0], -base[1], -base[2]) }

fn write_pos_list<'a>(positions: impl IntoIterator<Item = &'a Pos>, base: Pos) -> Nbt {
    Nbt::List(positions.into_iter().map(|pos| write_pos(sub(*pos, base))).collect())
}

/// `readPosList`: a position saved as zero is skipped.
fn read_pos_list(tag: &NbtCompound, key: &str, base: Pos) -> Vec<Pos> {
    tag.get(key).and_then(Nbt::as_list).unwrap_or(&[]).iter().filter_map(Nbt::as_compound).map(read_pos)
        .filter(|pos| *pos != [0, 0, 0]).map(|pos| offset(pos, base[0], base[1], base[2])).collect()
}

fn read_optional_pos(tag: &NbtCompound, key: &str, base: Pos) -> Option<Pos> {
    let pos = read_pos(tag.get(key)?.as_compound()?);
    Some(offset(pos, base[0], base[1], base[2]))
}

impl StructureData {
    /// `writeClientTag`: min, max and the tanks, relative to the controller.
    pub(crate) fn write_client_tag(&self, controller: Pos) -> NbtCompound {
        let mut tag = NbtCompound::new();
        tag.insert(TAG_MIN.to_owned(), write_pos(sub(self.min, controller)));
        tag.insert(TAG_MAX.to_owned(), write_pos(sub(self.max, controller)));
        tag.insert(TAG_TANKS.to_owned(), write_pos_list(&self.tanks, controller));
        tag
    }

    /// `writeToTag`: the client's data, the extra positions and where the inside check is.
    pub(crate) fn write_to_tag(&self, controller: Pos) -> NbtCompound {
        let mut tag = self.write_client_tag(controller);
        if !self.extra.is_empty() { tag.insert(TAG_EXTRA_POS.to_owned(), write_pos_list(&self.extra, controller)); }
        if let Some(check) = self.inside_check { tag.insert(TAG_INSIDE_CHECK.to_owned(), write_pos(sub(check, controller))); }
        tag
    }
}

/// How a heating structure is shaped and what its blocks are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shape { Smeltery, Foundry }

/// `MultiblockCuboid` with `HeatingStructureMultiblock`: one per controller, remembering its last result and the tanks it met.
#[derive(Clone, Debug)]
pub(crate) struct Multiblock {
    shape: Shape,
    controller: Pos,
    has_floor: bool,
    has_frame: bool,
    has_ceiling: bool,
    pub max_height: i32,
    inner_limit: i32,
    tanks: Vec<Pos>,
    pub last_result: MultiblockResult,
}

impl Multiblock {
    /// SmelteryMultiblock: floor and walls, no frame, no ceiling; 64 layers and a 14 wide inside.
    pub(crate) fn smeltery(controller: Pos) -> Self { Self::new(Shape::Smeltery, controller, true, false, false) }

    /// FoundryMultiblock: the same with a frame.
    pub(crate) fn foundry(controller: Pos) -> Self { Self::new(Shape::Foundry, controller, true, true, false) }

    fn new(shape: Shape, controller: Pos, has_floor: bool, has_frame: bool, has_ceiling: bool) -> Self {
        Self { shape, controller, has_floor, has_frame, has_ceiling, max_height: 64, inner_limit: 14, tanks: Vec::new(), last_result: MultiblockResult::error(None, NO_ATTEMPT) }
    }

    pub(crate) fn shape(&self) -> Shape { self.shape }

    fn create(&mut self, min: Pos, max: Pos, extra: BTreeSet<Pos>) -> StructureData {
        // remove any tanks that are out of bounds, possible one got added in a layer later declared invalid
        self.tanks.retain(|pos| within(*pos, min, max));
        StructureData::new(min, max, extra, self.has_floor, self.has_frame, self.has_ceiling, self.tanks.clone())
    }

    /// `createClient`.
    pub(crate) fn create_client(&self, min: Pos, max: Pos, tanks: Vec<Pos>) -> StructureData {
        StructureData::new(min, max, BTreeSet::new(), self.has_floor, self.has_frame, self.has_ceiling, tanks)
    }

    /// `readFromTag`: the saved structure, `None` without bounds.
    pub(crate) fn read_from_tag(&mut self, tag: &NbtCompound, controller: Pos) -> Option<StructureData> {
        self.tanks = read_pos_list(tag, TAG_TANKS, controller);
        let min = read_optional_pos(tag, TAG_MIN, controller)?;
        let max = read_optional_pos(tag, TAG_MAX, controller)?;
        let extra = read_pos_list(tag, TAG_EXTRA_POS, controller).into_iter().collect();
        let mut data = self.create(min, max, extra);
        data.inside_check = read_optional_pos(tag, TAG_INSIDE_CHECK, controller);
        Some(data)
    }

    // --- block checks

    fn inner(&self, world: Lookup, pos: Pos) -> bool { world(pos).is_some_and(|cell| cell.air) }

    /// `isInnerBlock`.
    pub(crate) fn is_inner_block(&self, world: Lookup, pos: Pos) -> bool { self.inner(world, pos) }

    fn valid_block(&mut self, world: Lookup, pos: Pos, side: Side, is_frame: bool) -> bool {
        // controller always is valid
        if pos == self.controller { return true; }
        // blocks need an in structure property, else we don't believe they support multiblocks
        let Some(cell) = world(pos) else { return false };
        let Some(in_structure) = cell.in_structure else { return false };
        // if it is currently in a structure, make sure it is in ours
        if in_structure && cell.foreign { return false; }
        // the floor has a smaller list; frame blocks count as walls
        if side == Side::Floor && !is_frame { return cell.floor; }
        // tanks are collected between the checks
        if cell.tank {
            self.tanks.push(pos);
            return true;
        }
        cell.wall
    }

    // --- detection

    fn outer_pos(&self, world: Lookup, mut pos: Pos, direction: Direction, limit: i32) -> Pos {
        let [dx, dy, dz] = step(direction);
        let mut i = 0;
        while i < limit && world(pos).is_some() && self.inner(world, pos) {
            pos = offset(pos, dx, dy, dz);
            i += 1;
        }
        pos
    }

    /// Whether every chunk of the region is loaded (`hasChunksAt`).
    fn loaded(world: Lookup, from: Pos, to: Pos) -> bool {
        for cx in (from[0] >> 4)..=(to[0] >> 4) {
            for cz in (from[2] >> 4)..=(to[2] >> 4) {
                // any block of the chunk inside the region stands for it
                let probe = [(cx * 16).clamp(from[0], to[0]), from[1], (cz * 16).clamp(from[2], to[2])];
                if world(probe).is_none() { return false; }
            }
        }
        true
    }

    fn detect_cap(&mut self, world: Lookup, from: Pos, to: Pos, side: Side) -> MultiblockResult {
        if !Self::loaded(world, from, to) { return MultiblockResult::error(None, NOT_LOADED); }
        let height = from[1];
        if self.has_frame {
            let key = if side == Side::Ceiling { INVALID_CEILING_FRAME } else { INVALID_FLOOR_FRAME };
            for x in from[0]..=to[0] {
                for z in [from[2], to[2]] {
                    if !self.valid_block(world, [x, height, z], side, true) { return MultiblockResult::error(Some([x, height, z]), key); }
                }
            }
            // z direction, corners were checked already
            for z in (from[2] + 1)..to[2] {
                for x in [from[0], to[0]] {
                    if !self.valid_block(world, [x, height, z], side, true) { return MultiblockResult::error(Some([x, height, z]), key); }
                }
            }
        }
        let key = if side == Side::Ceiling { INVALID_CEILING_BLOCK } else { INVALID_FLOOR_BLOCK };
        for z in (from[2] + 1)..to[2] {
            for x in (from[0] + 1)..to[0] {
                if !self.valid_block(world, [x, height, z], side, false) { return MultiblockResult::error(Some([x, height, z]), key); }
            }
        }
        MultiblockResult::success()
    }

    /// An inner layer, an area with an empty centre. The original collects blocks inside that are not air as extra positions, but its
    /// inner block test is "is air", so nothing inside can be both: `extra` stays empty for both structures.
    fn detect_layer(&mut self, world: Lookup, from: Pos, to: Pos) -> MultiblockResult {
        if !Self::loaded(world, from, to) { return MultiblockResult::error(None, NOT_LOADED); }
        let height = from[1];
        // the inside first, a quick exit for the ceiling and floor detection
        for x in (from[0] + 1)..to[0] {
            for z in (from[2] + 1)..to[2] {
                if !self.inner(world, [x, height, z]) { return MultiblockResult::error(Some([x, height, z]), INVALID_INNER_BLOCK); }
            }
        }
        if self.has_frame {
            for corner in [from, [from[0], height, to[2]], [to[0], height, from[2]], to] {
                if !self.valid_block(world, corner, Side::Wall, true) { return MultiblockResult::error(Some(corner), INVALID_WALL_FRAME); }
            }
        }
        for x in (from[0] + 1)..to[0] {
            for z in [from[2], to[2]] {
                if !self.valid_block(world, [x, height, z], Side::Wall, false) { return MultiblockResult::error(Some([x, height, z]), INVALID_WALL_BLOCK); }
            }
        }
        for z in (from[2] + 1)..to[2] {
            for x in [from[0], to[0]] {
                if !self.valid_block(world, [x, height, z], Side::Wall, false) { return MultiblockResult::error(Some([x, height, z]), INVALID_WALL_BLOCK); }
            }
        }
        MultiblockResult::success()
    }

    /// `detectMultiblock`: the structure around the controller facing `facing` (the controller is in a wall, the centre behind it), or
    /// `None` with `last_result` saying why not.
    pub(crate) fn detect(&mut self, world: Lookup, facing: Direction) -> Option<StructureData> {
        // clear tanks from the last check
        self.tanks.clear();
        // center is the block behind the controller
        let back = step(opposite(facing));
        let mut center = offset(self.controller, back[0], back[1], back[2]);
        // if behind the controller is not a valid inner block, behavior depends on the frame
        let mut needed_cap = None;
        if !self.inner(world, center) {
            // with no frame it is invalid; with a frame and no floor nor ceiling there is no way this block would be here
            if !self.has_frame || (!self.has_floor && !self.has_ceiling) {
                self.last_result = MultiblockResult::error(Some(center), INVALID_INNER_BLOCK);
                return None;
            }
            // with a frame we could be on one of the caps: pick the one we have, the ceiling when both
            if self.has_floor {
                needed_cap = Some(Side::Floor);
                center = offset(center, 0, 1, 0);
            } else {
                needed_cap = Some(Side::Ceiling);
                center = offset(center, 0, -1, 0);
            }
            if !self.inner(world, center) {
                self.last_result = MultiblockResult::error(Some(center), INVALID_INNER_BLOCK);
                return None;
            }
        }

        // distances to the edges including the outer blocks, per horizontal direction
        let mut edges = [0i32; 4];
        for (index, direction) in HORIZONTAL.into_iter().enumerate() {
            let pos = self.outer_pos(world, center, direction, self.inner_limit + 1);
            edges[index] = (pos[0] - center[0]) + (pos[2] - center[2]);
        }
        let (north, east, south, west) = (edges[0], edges[1], edges[2], edges[3]);

        // walls too far away? (the original names the south-north extent x)
        let xd = (south - north) - 1;
        let zd = (east - west) - 1;
        if xd > self.inner_limit || zd > self.inner_limit {
            self.last_result = MultiblockResult { success: false, pos: None, key: TOO_LARGE, args: vec![xd, zd, self.inner_limit, self.inner_limit] };
            return None;
        }

        let from = offset(center, west, 0, north);
        let to = offset(center, east, 0, south);
        let above = |pos: Pos, layers: i32| offset(pos, 0, layers, 0);

        // make sure the middle layer is valid
        let result = self.detect_layer(world, from, to);
        if !result.success {
            self.last_result = result;
            return None;
        }
        let mut layer_result = MultiblockResult::success();
        self.last_result = MultiblockResult::success();

        // iterate down until the bottom, unless the controller is in the bottom layer
        let mut min_layer = -1;
        let mut remaining_height = self.max_height - 1;
        if needed_cap != Some(Side::Floor) {
            while min_layer > -remaining_height {
                layer_result = self.detect_layer(world, above(from, min_layer), above(to, min_layer));
                if !layer_result.success { break; }
                min_layer -= 1;
            }
        }
        remaining_height += min_layer + 1;
        if self.has_floor {
            let floor = self.detect_cap(world, above(from, min_layer), above(to, min_layer), Side::Floor);
            if !floor.success {
                self.last_result = floor;
                return None;
            }
        } else {
            min_layer += 1;
            self.last_result = layer_result.clone();
        }

        // go up to find the ceiling, again unless we started in it
        let mut max_layer = 1;
        if needed_cap != Some(Side::Ceiling) {
            while max_layer < remaining_height {
                layer_result = self.detect_layer(world, above(from, max_layer), above(to, max_layer));
                if !layer_result.success { break; }
                max_layer += 1;
            }
        }
        if self.has_ceiling {
            let ceiling = self.detect_cap(world, above(from, max_layer), above(to, max_layer), Side::Ceiling);
            if !ceiling.success {
                self.last_result = ceiling;
                return None;
            }
        } else {
            max_layer -= 1;
            self.last_result = layer_result;
        }

        Some(self.create(above(from, min_layer), above(to, max_layer), BTreeSet::new()))
    }

    /// `canExpand`: whether one more layer above the structure would be valid (a floorless structure grows downwards instead).
    pub(crate) fn can_expand(&mut self, data: &StructureData, world: Lookup) -> bool {
        let (from, to) = if self.has_floor {
            let to = offset(data.max, 0, 1, 0);
            ([data.min[0], to[1], data.min[2]], to)
        } else {
            let from = offset(data.min, 0, -1, 0);
            (from, [data.max[0], from[1], data.max[2]])
        };
        let result = self.detect_layer(world, from, to);
        self.last_result = result.clone();
        result.success
    }

    /// `shouldUpdate`: whether a changed block at `pos` (now `cell`) means the structure must be checked again.
    pub(crate) fn should_update(&self, structure: &StructureData, pos: Pos, cell: &Cell) -> bool {
        if structure.within_bounds(pos) {
            // a part of the structure must still be a valid structure block
            if structure.contains(pos) { return !cell.any; }
            // not part of the structure: only a block that is not air inside matters, a block added to the frame does not
            return structure.is_inside(pos) && !cell.air;
        }
        // one block above, it might be trying to expand upwards
        structure.is_directly_above(pos) && cell.wall
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A world of cells; everything not set is loaded air, everything in `unloaded` has no chunk.
    #[derive(Default)]
    struct World { cells: HashMap<Pos, Cell>, unloaded: bool }

    impl World {
        fn lookup(&self) -> impl Fn(Pos) -> Option<Cell> + '_ {
            move |pos| {
                if self.unloaded && pos[0] >= 16 { return None; }
                Some(self.cells.get(&pos).cloned().unwrap_or(Cell { air: true, ..Cell::default() }))
            }
        }
        fn block(&mut self, pos: Pos, floor: bool, tank: bool, wall: bool) {
            self.cells.insert(pos, Cell { in_structure: Some(false), any: true, floor, tank, wall, ..Cell::default() });
        }
        /// Seared bricks: floor and wall alike.
        fn bricks(&mut self, pos: Pos) { self.block(pos, true, false, true); }
    }

    /// A smeltery with a 3 x 3 inside and `height` layers: floor at y 0, the wall ring above it, corners left empty (no frame).
    fn smeltery(height: i32) -> World {
        let mut world = World::default();
        for x in 0..5 { for z in 0..5 { world.bricks([x, 0, z]); } }
        for y in 1..=height {
            for i in 1..4 {
                for pos in [[i, y, 0], [i, y, 4], [0, y, i], [4, y, i]] { world.bricks(pos); }
            }
        }
        world
    }

    const CONTROLLER: Pos = [2, 2, 0];

    #[test]
    fn a_smeltery_around_a_controller_facing_the_inside_is_found_layer_by_layer() {
        let world = smeltery(3);
        let mut multiblock = Multiblock::smeltery(CONTROLLER);
        let lookup = world.lookup();
        let data = multiblock.detect(&lookup, Direction::North).expect("forms");
        // the result is the layer above the structure that could not be added: the reason it does not grow (the expand marker)
        assert_eq!((multiblock.last_result.key, multiblock.last_result.pos), (INVALID_WALL_BLOCK, Some([1, 4, 0])));
        assert_eq!((data.min, data.max), ([0, 0, 0], [4, 3, 4]));
        assert_eq!((data.min_inside, data.max_inside), ([1, 1, 1], [3, 3, 3]));
        assert_eq!((data.inner_x, data.inner_y, data.inner_z), (3, 3, 3));
        assert_eq!(data.bounds, [1.0, 1.0, 1.0, 4.0, 4.0, 4.0]);
        // the box minus the inside and the 16 floor ring and 12 corner blocks no frame leaves out: 9 floor and 36 wall blocks
        assert_eq!(data.contained().len(), 45);
        assert!(data.contains([2, 0, 2]) && data.contains([0, 2, 2]) && !data.contains([0, 0, 0]) && !data.contains([0, 3, 0]) && !data.contains([2, 2, 2]));
        // a smeltery of this size: perimeter (2 * 2 * 2 + 2 * 2 * 2 + 2 * 2) = 20
        assert_eq!(data.perimeter_count(), 20);
    }

    #[test]
    fn a_controller_in_the_south_wall_forms_only_when_it_faces_out_of_the_structure() {
        // the same world with the controller in the z-max wall instead: facing south (out of the structure,
        // toward where a placer stands) finds the inside behind it, while facing north (into the structure)
        // scans the open air in front of it and finds no wall within the limit, as the original does
        let world = smeltery(3);
        let lookup = world.lookup();
        let mut outward = Multiblock::smeltery([2, 2, 4]);
        let data = outward.detect(&lookup, Direction::South).expect("forms facing out");
        assert_eq!((data.min, data.max), ([0, 0, 0], [4, 3, 4]));
        assert_eq!(data.contained().len(), 45);
        let mut inward = Multiblock::smeltery([2, 2, 4]);
        assert!(inward.detect(&lookup, Direction::North).is_none());
        assert_eq!(inward.last_result.key, TOO_LARGE);
    }

    #[test]
    fn a_tank_in_a_floor_corner_is_neither_collected_nor_part_of_the_structure() {
        // a floor corner touches two edges and the floor, so it sits on the frame: the original's
        // containsBase leaves it out (its in_structure stays false) and detection never visits the
        // corner, so it is not collected as a tank either; the structure still forms around it
        let mut world = smeltery(3);
        world.block([0, 0, 0], true, true, true);
        let mut multiblock = Multiblock::smeltery(CONTROLLER);
        let data = multiblock.detect(&world.lookup(), Direction::North).expect("forms");
        assert!(data.tanks.is_empty());
        assert!(!data.contains([0, 0, 0]));
    }

    #[test]
    fn walls_taller_than_the_controller_extend_the_structure_and_it_stops_at_the_first_open_layer() {
        let mut world = smeltery(5);
        // the controller sits in the middle layer; a hole in the wall at y 4 ends the structure at y 3
        world.cells.remove(&[4, 4, 2]);
        let mut multiblock = Multiblock::smeltery(CONTROLLER);
        let lookup = world.lookup();
        let data = multiblock.detect(&lookup, Direction::North).expect("forms");
        assert_eq!((data.min[1], data.max[1]), (0, 3));
        assert_eq!(multiblock.last_result.key, INVALID_WALL_BLOCK, "the layer that stopped it is the result, as the expand check reports it");
        assert_eq!(multiblock.last_result.pos, Some([4, 4, 2]));
        // the layer above is not valid, so the structure cannot grow
        assert!(!multiblock.can_expand(&data, &lookup));
        let mut mended = smeltery(5);
        mended.cells.remove(&[4, 4, 2]);
        mended.bricks([4, 4, 2]);
        let lookup = mended.lookup();
        assert!(multiblock.can_expand(&data, &lookup), "with the wall mended one more layer fits");
        assert!(multiblock.last_result.success);
    }

    #[test]
    fn the_reason_a_structure_does_not_form_names_the_block_to_blame() {
        let mut multiblock = Multiblock::smeltery(CONTROLLER);
        // no floor block under the middle
        let mut world = smeltery(3);
        world.cells.remove(&[2, 0, 2]);
        assert!(multiblock.detect(&world.lookup(), Direction::North).is_none());
        assert_eq!((multiblock.last_result.key, multiblock.last_result.pos), (INVALID_FLOOR_BLOCK, Some([2, 0, 2])));
        // a wall block of the wrong kind: seared glass is a wall block but not in the floor list
        let mut world = smeltery(3);
        world.block([2, 0, 3], false, false, true);
        assert!(multiblock.detect(&world.lookup(), Direction::North).is_none());
        assert_eq!(multiblock.last_result.pos, Some([2, 0, 3]));
        // something below the middle layer ends the downward search there, and then it is the floor that fails
        let mut world = smeltery(3);
        world.cells.insert([2, 1, 2], Cell { air: false, ..Cell::default() });
        assert!(multiblock.detect(&world.lookup(), Direction::North).is_none());
        assert_eq!((multiblock.last_result.key, multiblock.last_result.pos), (INVALID_FLOOR_BLOCK, Some([1, 1, 1])));
    }

    #[test]
    fn a_block_behind_the_controller_a_foreign_servant_and_a_plain_block_all_stop_the_structure() {
        let mut multiblock = Multiblock::smeltery(CONTROLLER);
        let mut world = smeltery(3);
        // the centre behind the controller is not air
        world.cells.insert([2, 2, -1], Cell::default());
        assert!(multiblock.detect(&world.lookup(), Direction::South).is_none());
        assert_eq!((multiblock.last_result.key, multiblock.last_result.pos), (INVALID_INNER_BLOCK, Some([2, 2, -1])));
        // a wall block in a structure that belongs to someone else
        world.cells.get_mut(&[0, 2, 2]).unwrap().in_structure = Some(true);
        world.cells.get_mut(&[0, 2, 2]).unwrap().foreign = true;
        assert!(multiblock.detect(&world.lookup(), Direction::North).is_none());
        assert_eq!((multiblock.last_result.key, multiblock.last_result.pos), (INVALID_WALL_BLOCK, Some([0, 2, 2])));
        // but one that is in a structure and ours (a servant that accepts the controller) is fine
        world.cells.get_mut(&[0, 2, 2]).unwrap().foreign = false;
        assert!(multiblock.detect(&world.lookup(), Direction::North).is_some());
        // and a block with no in_structure property is not a structure block at all
        world.cells.get_mut(&[0, 2, 2]).unwrap().in_structure = None;
        assert!(multiblock.detect(&world.lookup(), Direction::North).is_none());
    }

    #[test]
    fn tanks_are_collected_in_detection_order_and_the_unloaded_edge_of_the_world_says_so() {
        let mut world = smeltery(3);
        world.block([4, 2, 2], false, true, true);
        world.block([2, 3, 4], false, true, true);
        let mut multiblock = Multiblock::smeltery(CONTROLLER);
        let data = multiblock.detect(&world.lookup(), Direction::North).expect("forms");
        assert_eq!(data.tanks, vec![[4, 2, 2], [2, 3, 4]], "the middle layer is checked first, then the layers below, then above");
        assert!(data.has_tanks());
        // the same structure moved so its east wall is in a chunk that is not loaded
        let base = smeltery(3);
        let mut moved = World { cells: HashMap::new(), unloaded: true };
        for (pos, cell) in &base.cells { moved.cells.insert([pos[0] + 13, pos[1], pos[2]], cell.clone()); }
        let mut edge = Multiblock::smeltery([15, 2, 0]);
        assert!(edge.detect(&moved.lookup(), Direction::North).is_none());
        assert_eq!(edge.last_result.key, NOT_LOADED);
    }

    #[test]
    fn an_inside_wider_than_fourteen_is_too_large() {
        // a 16 wide open floor with walls only far to the sides: the search along x stops at the limit
        let mut world = World::default();
        for x in 0..18 { for z in 0..5 { world.bricks([x, 0, z]); } }
        for y in 1..=3 { for z in 1..4 { world.bricks([0, y, z]); world.bricks([17, y, z]); } }
        for y in 1..=3 { for x in 1..17 { world.bricks([x, y, 0]); world.bricks([x, y, 4]); } }
        let mut multiblock = Multiblock::smeltery([8, 2, 0]);
        assert!(multiblock.detect(&world.lookup(), Direction::North).is_none());
        assert_eq!((multiblock.last_result.key, multiblock.last_result.pos, multiblock.last_result.args.clone()), (TOO_LARGE, None, vec![3, 16, 14, 14]));
    }

    /// A foundry: a frame, inside 3 x 3, the floor and two wall layers.
    fn foundry() -> World {
        let mut world = World::default();
        for x in 0..5 { for z in 0..5 { world.bricks([x, 0, z]); } }
        for y in 1..=2 {
            for x in 0..5 { for z in 0..5 {
                if x == 0 || x == 4 || z == 0 || z == 4 { world.bricks([x, y, z]); }
            } }
        }
        world
    }

    #[test]
    fn the_foundry_counts_its_frame_and_a_controller_in_the_floor_row_finds_its_inside_above() {
        let world = foundry();
        let mut multiblock = Multiblock::foundry([2, 1, 0]);
        let data = multiblock.detect(&world.lookup(), Direction::North).expect("forms");
        assert_eq!((data.min, data.max), ([0, 0, 0], [4, 2, 4]));
        assert_eq!((data.inner_x, data.inner_y, data.inner_z), (3, 2, 3));
        assert_eq!(data.contained().len(), 5 * 3 * 5 - 3 * 2 * 3, "with a frame the whole cube but the inside is the structure");
        assert!(data.contains([0, 0, 0]) && data.contains([0, 2, 0]) && !data.contains([2, 1, 2]));
        // a controller in the floor row, behind it a floor block: the cap logic moves the centre up one
        let mut floor_row = Multiblock::foundry([2, 0, 0]);
        let data = floor_row.detect(&world.lookup(), Direction::North).expect("forms from the floor row");
        assert_eq!((data.min, data.max), ([0, 0, 0], [4, 2, 4]));
        // the frame corner of the middle layer is named when it is missing
        let mut broken = foundry();
        broken.cells.remove(&[4, 1, 4]);
        assert!(multiblock.detect(&broken.lookup(), Direction::North).is_none());
        assert_eq!((multiblock.last_result.key, multiblock.last_result.pos), (INVALID_WALL_FRAME, Some([4, 1, 4])));
        // a missing corner of the layer above only ends the structure one layer lower
        let mut short = foundry();
        short.cells.remove(&[4, 2, 4]);
        let data = multiblock.detect(&short.lookup(), Direction::North).expect("forms");
        assert_eq!(data.max[1], 1);
        // a missing floor corner is a floor frame error
        let mut open = foundry();
        open.cells.remove(&[0, 0, 0]);
        assert!(multiblock.detect(&open.lookup(), Direction::North).is_none());
        assert_eq!((multiblock.last_result.key, multiblock.last_result.pos), (INVALID_FLOOR_FRAME, Some([0, 0, 0])));
    }

    #[test]
    fn a_changed_block_asks_for_a_new_check_only_when_it_could_change_the_structure() {
        let world = smeltery(3);
        let mut multiblock = Multiblock::smeltery(CONTROLLER);
        let data = multiblock.detect(&world.lookup(), Direction::North).unwrap();
        let air = Cell { air: true, ..Cell::default() };
        let wall = Cell { any: true, wall: true, floor: true, in_structure: Some(true), ..Cell::default() };
        let junk = Cell::default();
        // a part of the structure turning into air or junk, or staying a seared block
        assert!(multiblock.should_update(&data, [0, 2, 2], &air));
        assert!(multiblock.should_update(&data, [0, 2, 2], &junk));
        assert!(!multiblock.should_update(&data, [0, 2, 2], &wall));
        // inside: anything but air; a frame corner of a frameless structure is not part of it
        assert!(multiblock.should_update(&data, [2, 2, 2], &junk));
        assert!(!multiblock.should_update(&data, [2, 2, 2], &air));
        assert!(!multiblock.should_update(&data, [0, 0, 0], &junk));
        // one above: only a wall block could expand it
        assert!(multiblock.should_update(&data, [2, 4, 2], &wall));
        assert!(!multiblock.should_update(&data, [2, 4, 2], &junk));
        assert!(!multiblock.should_update(&data, [20, 4, 2], &wall));
    }

    #[test]
    fn the_inside_check_walks_every_inner_block_and_the_structure_saves_and_reloads() {
        let world = smeltery(3);
        let mut multiblock = Multiblock::smeltery(CONTROLLER);
        let mut data = multiblock.detect(&world.lookup(), Direction::North).unwrap();
        let walk: Vec<Pos> = (0..28).map(|_| data.next_inside_check()).collect();
        assert_eq!(walk[0], [1, 1, 1]);
        assert_eq!(walk[1], [1, 1, 2]);
        assert_eq!(walk[3], [2, 1, 1], "after a row of z comes the next x");
        assert_eq!(walk[9], [1, 2, 1], "after a layer comes the next one up");
        assert_eq!(walk[26], [3, 3, 3]);
        assert_eq!(walk[27], [1, 1, 1], "and it wraps around");
        // saved relative to the controller; the inside check position is part of the save, extra positions only when present
        data.extra.insert([2, 1, 2]);
        let tag = data.write_to_tag(CONTROLLER);
        let mut fresh = Multiblock::smeltery(CONTROLLER);
        let loaded = fresh.read_from_tag(&tag, CONTROLLER).expect("reads");
        assert_eq!(loaded, data);
        let client = data.client();
        assert!(client.extra.is_empty() && client.min == data.min && client.tanks == data.tanks);
        assert!(fresh.read_from_tag(&NbtCompound::new(), CONTROLLER).is_none(), "no bounds, no structure");
    }

    #[test]
    fn joining_and_leaving_a_structure_only_touches_blocks_that_changed_sides() {
        let world = smeltery(3);
        let mut multiblock = Multiblock::smeltery(CONTROLLER);
        let old = multiblock.detect(&world.lookup(), Direction::North).unwrap();
        let taller = smeltery(4);
        let new = Multiblock::smeltery(CONTROLLER).detect(&taller.lookup(), Direction::North).unwrap();
        let mut log = Vec::new();
        new.assign_master(Some(&old), &mut |pos, joined| log.push((pos, joined)));
        assert_eq!(log.len(), 12, "only the new top ring joins: 12 wall blocks");
        assert!(log.iter().all(|(pos, joined)| *joined && pos[1] == 4));
        let mut first = Vec::new();
        old.assign_master(None, &mut |pos, joined| first.push((pos, joined)));
        assert_eq!(first.len(), 45);
        let mut shrunk = Vec::new();
        old.assign_master(Some(&new), &mut |pos, joined| shrunk.push((pos, joined)));
        assert_eq!(shrunk.len(), 12, "going back to the smaller structure frees the top ring and nothing else");
        assert!(shrunk.iter().all(|(pos, joined)| !*joined && pos[1] == 4));
        let mut freed = 0;
        new.clear_master(&mut |_, joined| { assert!(!joined); freed += 1; });
        assert_eq!(freed, 57);
    }
}
