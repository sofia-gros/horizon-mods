//! The area-of-effect iterators of tool definitions (ToolHooks.AOE_ITERATOR): given the block a tool
//! breaks, the extra blocks it takes with it. Box, circle, vein, conditional and tree iterators, decoded
//! from the definition modules and evaluated against block states read through the world handle.
//!
//! The iterators select by geometry and their own block rules; the original's default block
//! predicate (AreaOfEffectIterator.defaultBlockPredicate) is the caller's to apply, as the
//! original applies it inside its box and circle iterators: it rejects air, blocks whose
//! hardness is -1 or more than three times the origin's, and blocks the tool is not effective
//! on. Which iterators owe it is `Iterator::owes_default_predicate`'s to say: box and circle
//! owe it, a conditional owes what the branch it takes owes (ConditionalAOEIterator
//! delegates to the branch's getBlocks, predicate included), and vein and tree owe nothing.
//! The hardness half reads the hardness the game published from its block table
//! (pluto_lib::world_api_v1::block_hardness). SlimeKnights (MIT; see LICENSE).
use pluto_lib::{Direction, Identifier};
use serde_json::{Map, Value};
use std::collections::{HashSet, VecDeque};

/// BoxSize: an extent along the width, height and depth axes.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(crate) struct BoxSize { pub width: i32, pub height: i32, pub depth: i32 }

impl BoxSize {
    fn is_zero(self) -> bool { self.width == 0 && self.height == 0 && self.depth == 0 }
    fn decode(value: Option<&Value>) -> Result<Self, String> {
        let Some(value) = value else { return Ok(Self::default()); };
        let object = value.as_object().ok_or("box size must be an object")?;
        let field = |key: &str| -> Result<i32, String> {
            match object.get(key) {
                None | Some(Value::Null) => Ok(0),
                Some(Value::Number(number)) => {
                    let value = crate::materials::number_int(&number.to_string());
                    if value < 0 { Err(format!("box size {key} must be nonnegative")) } else { Ok(value) }
                }
                _ => Err(format!("box size {key} must be numeric")),
            }
        };
        Ok(Self { width: field("width")?, height: field("height")?, depth: field("depth")? })
    }
}

/// IBoxExpansion: how a rectangle's axes are chosen from the hit face and the player's look.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Expansion { SideHit, Pitch, Height }

impl Expansion {
    fn parse(name: &str) -> Result<Self, String> {
        match name {
            "tconstruct:side_hit" => Ok(Self::SideHit),
            "tconstruct:pitch" => Ok(Self::Pitch),
            "tconstruct:height" => Ok(Self::Height),
            _ => Err(format!("unknown box expansion {name}")),
        }
    }
}

/// One AOE iterator, as AreaOfEffectIterator.Loadable decodes it.
#[derive(Clone, Debug)]
pub(crate) enum Iterator {
    /// BoxAOEIterator.
    Box { bonus: BoxSize, expansions: Vec<BoxSize>, direction: Expansion },
    /// CircleAOEIterator.
    Circle { diameter: i32, three_d: bool },
    /// VeiningAOEIterator.
    Vein { max_distance: i32 },
    /// ConditionalAOEIterator.
    Conditional { condition: crate::mining_speed::BlockCondition, if_true: Box<Iterator>, if_false: Box<Iterator> },
    /// TreeAOEIterator.
    Tree { width_bonus: i32, depth_bonus: i32 },
    /// The empty iterator (a conditional's missing branch).
    Empty,
}

/// The block at a position, for the iterators that compare blocks; None when nothing is loaded there.
pub(crate) type BlockAt<'a> = dyn Fn([i32; 3]) -> Option<Identifier> + 'a;
/// Whether the block at a position is in a tag.
pub(crate) type InTagAt<'a> = dyn Fn([i32; 3], &Identifier) -> bool + 'a;

fn int_field(fields: &Map<String, Value>, key: &str, default: i32, minimum: i32) -> Result<i32, String> {
    match fields.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Number(number)) => {
            let value = crate::materials::number_int(&number.to_string());
            if value < minimum { Err(format!("{key} must be at least {minimum}")) } else { Ok(value) }
        }
        _ => Err(format!("{key} must be numeric")),
    }
}

impl Iterator {
    /// Decodes one iterator object (a definition module or a conditional's branch). Ok(None) when a
    /// nested branch names an iterator kind this port does not implement: the caller records the whole
    /// module as unbound rather than guessing at its shape.
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let kind = fields.get("type").and_then(Value::as_str).ok_or("iterator type is required")?;
        let decoded = match kind {
            "tconstruct:box_aoe" => {
                let expansions = match fields.get("expansions") {
                    None | Some(Value::Null) => Vec::new(),
                    Some(Value::Array(list)) => list.iter().map(|size| BoxSize::decode(Some(size))).collect::<Result<Vec<_>, String>>()?,
                    _ => return Err("expansions must be an array".into()),
                };
                let direction = fields.get("expansion_direction").and_then(Value::as_str).ok_or("expansion_direction is required")?;
                Self::Box { bonus: BoxSize::decode(fields.get("bonus"))?, expansions, direction: Expansion::parse(direction)? }
            }
            "tconstruct:circle_aoe" => {
                let three_d = match fields.get("3D") {
                    None | Some(Value::Null) => false,
                    Some(Value::Bool(value)) => *value,
                    _ => return Err("3D must be a boolean".into()),
                };
                Self::Circle { diameter: int_field(fields, "diameter", 1, 1)?, three_d }
            }
            "tconstruct:vein_aoe" => Self::Vein { max_distance: int_field(fields, "max_distance", 0, 0)? },
            "tconstruct:conditional_aoe" => {
                let condition = fields.get("condition").ok_or("condition is required")?;
                let condition = crate::mining_speed::BlockCondition::decode(condition)?.ok_or("condition must not be empty")?;
                let branch = |key: &str| -> Result<Option<Box<Iterator>>, String> {
                    match fields.get(key) {
                        None | Some(Value::Null) => Ok(None),
                        Some(Value::Object(object)) => Ok(Self::decode(object)?.map(Box::new)),
                        _ => Err(format!("{key} must be an iterator object")),
                    }
                };
                let if_true = branch("if_true")?;
                let Some(if_true) = if_true else { return Ok(None); };
                let if_false = branch("if_false")?.unwrap_or_else(|| Box::new(Self::Empty));
                Self::Conditional { condition, if_true, if_false }
            }
            "tconstruct:tree_aoe" => Self::Tree {
                width_bonus: int_field(fields, "width_bonus", 0, 0)?,
                depth_bonus: int_field(fields, "depth_bonus", 0, 0)?,
            },
            "tconstruct:empty" => Self::Empty,
            _ => return Ok(None),
        };
        Ok(Some(decoded))
    }

    /// The extra positions this iterator selects around `origin`, in the original's iteration order.
    /// `expanded` is the tool's EXPANDED volatile level; `face` the hit face; `yaw`/`pitch` the player's
    /// look. Box and circle positions still owe the caller the default block
    /// predicate (see the module notes), as does a conditional through the branch it takes;
    /// vein and tree apply their own block rules here.
    pub(crate) fn positions(&self, origin: [i32; 3], face: Direction, yaw: f32, pitch: f32, expanded: i32, block_at: &BlockAt<'_>, in_tag_at: &InTagAt<'_>) -> Vec<[i32; 3]> {
        match self {
            Self::Empty => Vec::new(),
            Self::Box { bonus, expansions, direction } => {
                let size = size_for(*bonus, expansions, expanded);
                if size.is_zero() { return Vec::new(); }
                let (width, height, depth, traverse_down) = axes(*direction, face, yaw, pitch);
                rectangle(origin, width, size.width, height, size.height, traverse_down, depth, size.depth)
            }
            Self::Circle { diameter, three_d } => {
                let diameter = diameter + expanded;
                if diameter <= 1 { return Vec::new(); }
                let extra = diameter / 2;
                let radius_sq = diameter * diameter / 4;
                let (width, height, depth, _) = axes(Expansion::SideHit, face, yaw, pitch);
                rectangle(origin, width, extra, height, extra, true, depth, if *three_d { extra } else { 0 })
                    .into_iter().filter(|pos| distance_sq(origin, *pos) <= radius_sq).collect()
            }
            Self::Vein { max_distance } => vein(origin, *max_distance + expanded, block_at),
            Self::Conditional { .. } => match self.chosen(origin, block_at, in_tag_at) {
                Some(chosen) => chosen.positions(origin, face, yaw, pitch, expanded, block_at, in_tag_at),
                None => Vec::new(),
            },
            Self::Tree { width_bonus, depth_bonus } => {
                tree(origin, face, yaw, *width_bonus + (expanded + 1) / 2, *depth_bonus + expanded / 2, block_at, in_tag_at)
            }
        }
    }

    /// The branch a conditional takes for the block at `origin`: None when this is not a
    /// conditional, or when no block is loaded at the origin (the conditional then selects
    /// nothing). The branch is picked without a dig report: no block facts are known at
    /// selection time, so a block-property predicate takes its fallback (blast resistance)
    /// or stays unmatched (requires-tool), exactly as a dig that reported nothing does.
    fn chosen(&self, origin: [i32; 3], block_at: &BlockAt<'_>, in_tag_at: &InTagAt<'_>) -> Option<&Self> {
        let Self::Conditional { condition, if_true, if_false } = self else { return None; };
        let block = block_at(origin)?;
        if condition.matches(&block, &|tag| in_tag_at(origin, tag), &crate::mining_speed::BlockFacts::default()) { Some(if_true) } else { Some(if_false) }
    }

    /// Whether the positions this iterator selects still owe the caller the default block
    /// predicate (AreaOfEffectIterator.defaultBlockPredicate). Box and circle owe it, as
    /// their getBlocks applies it; vein, tree and empty select by their own block rules and
    /// owe nothing. A conditional owes what the branch it takes owes, as
    /// ConditionalAOEIterator.getBlocks delegates to the branch's getBlocks, predicate
    /// included; a conditional whose origin block is not loaded selects nothing and owes
    /// nothing.
    pub(crate) fn owes_default_predicate(&self, origin: [i32; 3], block_at: &BlockAt<'_>, in_tag_at: &InTagAt<'_>) -> bool {
        match self {
            Self::Box { .. } | Self::Circle { .. } => true,
            Self::Conditional { .. } => self.chosen(origin, block_at, in_tag_at).is_some_and(|branch| branch.owes_default_predicate(origin, block_at, in_tag_at)),
            _ => false,
        }
    }
}

/// BoxAOEIterator.sizeFor: full expansion sets cycle, the remainder applies once.
fn size_for(base: BoxSize, expansions: &[BoxSize], level: i32) -> BoxSize {
    let count = expansions.len() as i32;
    if level == 0 || count == 0 { return base; }
    let mut size = base;
    if level >= count {
        let cycles = level / count;
        for expansion in expansions {
            size.width += expansion.width * cycles;
            size.height += expansion.height * cycles;
            size.depth += expansion.depth * cycles;
        }
    }
    for expansion in &expansions[..(level % count) as usize] {
        size.width += expansion.width;
        size.height += expansion.height;
        size.depth += expansion.depth;
    }
    size
}

fn vec_of(direction: Direction) -> [i32; 3] {
    match direction {
        Direction::Down => [0, -1, 0],
        Direction::Up => [0, 1, 0],
        Direction::North => [0, 0, -1],
        Direction::South => [0, 0, 1],
        Direction::West => [-1, 0, 0],
        Direction::East => [1, 0, 0],
    }
}

fn opposite(direction: Direction) -> Direction {
    match direction {
        Direction::Down => Direction::Up,
        Direction::Up => Direction::Down,
        Direction::North => Direction::South,
        Direction::South => Direction::North,
        Direction::West => Direction::East,
        Direction::East => Direction::West,
    }
}

/// Direction.getClockWise in the horizontal plane (north to east to south to west).
fn clockwise(direction: Direction) -> Direction {
    match direction {
        Direction::North => Direction::East,
        Direction::East => Direction::South,
        Direction::South => Direction::West,
        Direction::West => Direction::North,
        other => other,
    }
}

fn counter_clockwise(direction: Direction) -> Direction {
    match direction {
        Direction::North => Direction::West,
        Direction::West => Direction::South,
        Direction::South => Direction::East,
        Direction::East => Direction::North,
        other => other,
    }
}

fn is_vertical(direction: Direction) -> bool { matches!(direction, Direction::Down | Direction::Up) }

/// The player's horizontal facing from their yaw (Direction.fromYRot).
fn facing(yaw: f32) -> Direction {
    match (yaw / 90.0 + 0.5).floor() as i32 & 3 {
        0 => Direction::South,
        1 => Direction::West,
        2 => Direction::North,
        _ => Direction::East,
    }
}

/// IBoxExpansion.getDirections: the (width, height, depth, traverseDown) axes for a hit.
fn axes(expansion: Expansion, face: Direction, yaw: f32, pitch: f32) -> ([i32; 3], [i32; 3], [i32; 3], bool) {
    let look = facing(yaw);
    let (width, height, depth, traverse_down) = match expansion {
        Expansion::SideHit => {
            let depth = opposite(face);
            let (width, height) = if is_vertical(face) {
                (clockwise(look), look)
            } else {
                (counter_clockwise(face), Direction::Up)
            };
            (width, height, depth, true)
        }
        Expansion::Pitch => {
            let width = clockwise(look);
            let (height, depth) = if pitch < -60.0 {
                (look, Direction::Up)
            } else if pitch > 60.0 {
                (look, Direction::Down)
            } else {
                (Direction::Up, look)
            };
            (width, height, depth, true)
        }
        Expansion::Height => {
            let depth = if is_vertical(face) { look } else { opposite(face) };
            (clockwise(depth), Direction::Up, depth, false)
        }
    };
    (vec_of(width), vec_of(height), vec_of(depth), traverse_down)
}

fn add(pos: [i32; 3], delta: [i32; 3], scale: i32) -> [i32; 3] {
    [pos[0] + delta[0] * scale, pos[1] + delta[1] * scale, pos[2] + delta[2] * scale]
}

fn distance_sq(a: [i32; 3], b: [i32; 3]) -> i32 {
    let (x, y, z) = (a[0] - b[0], a[1] - b[1], a[2] - b[2]);
    x * x + y * y + z * z
}

/// RectangleIterator: the solid around the origin (the origin itself excluded), width fastest.
fn rectangle(origin: [i32; 3], width: [i32; 3], extra_width: i32, height: [i32; 3], extra_height: i32, traverse_down: bool, depth: [i32; 3], extra_depth: i32) -> Vec<[i32; 3]> {
    let max_width = extra_width * 2;
    let max_height = if traverse_down { extra_height * 2 } else { extra_height };
    let mut out = Vec::new();
    for d in 0..=extra_depth {
        for h in 0..=max_height {
            for w in 0..=max_width {
                let pos = add(add(add(origin, width, w - extra_width), height, if traverse_down { h - extra_height } else { h }), depth, d);
                if pos != origin { out.push(pos); }
            }
        }
    }
    out
}

/// VeiningAOEIterator: breadth-first over positions holding the origin's block, out to `max_distance`
/// steps (neighbour order is Direction order: down, up, north, south, west, east).
fn vein(origin: [i32; 3], max_distance: i32, block_at: &BlockAt<'_>) -> Vec<[i32; 3]> {
    let Some(filter) = block_at(origin) else { return Vec::new(); };
    let mut visited: HashSet<[i32; 3]> = HashSet::from([origin]);
    let mut queue: VecDeque<([i32; 3], i32)> = VecDeque::from([(origin, 0)]);
    let mut out = Vec::new();
    while let Some((pos, distance)) = queue.pop_front() {
        if distance > 0 { out.push(pos); }
        if distance >= max_distance { continue; }
        for direction in Direction::ALL {
            let next = add(pos, vec_of(direction), 1);
            if visited.insert(next) && block_at(next).as_ref() == Some(&filter) {
                queue.push_back((next, distance + 1));
            }
        }
    }
    out
}

/// TreeAOEIterator.calculate: the trunk axes from the hit face and the player's facing.
fn tree(origin: [i32; 3], face: Direction, yaw: f32, extra_width: i32, extra_depth: i32, block_at: &BlockAt<'_>, in_tag_at: &InTagAt<'_>) -> Vec<[i32; 3]> {
    let Some(filter) = block_at(origin) else { return Vec::new(); };
    let (width_dir, depth_dir) = if extra_depth > 0 || extra_width > 0 {
        let depth = if is_vertical(face) { facing(yaw) } else { opposite(face) };
        (clockwise(depth), depth)
    } else {
        (Direction::Up, Direction::Up)
    };
    TreeWalk::new(origin, filter, width_dir, extra_width, depth_dir, extra_depth, block_at, in_tag_at).run()
}

/// TreeAOEIterator.TreeIterator, ported step for step: a queue of trunk positions climbing while the
/// block matches, sprouting branches that were validated on enqueue. The one input with no surface here
/// is BlockState.canOcclude for the block below a branch candidate; it is approximated as "some other
/// block is there" — air, leaves and the tree's own block read as non-occluding, everything else as
/// occluding — which diverges only for branches resting on partial blocks.
struct TreeWalk<'a> {
    block_at: &'a BlockAt<'a>,
    in_tag_at: &'a InTagAt<'a>,
    filter: Identifier,
    min_x: i32,
    max_x: i32,
    min_z: i32,
    max_z: i32,
    branch_visited: HashSet<[i32; 3]>,
    queue: VecDeque<TreePos>,
}

#[derive(Clone, Copy)]
struct TreePos {
    pos: [i32; 3],
    /// Up for a trunk position; the branch's direction otherwise.
    direction: Direction,
    checked: bool,
}

impl<'a> TreeWalk<'a> {
    fn new(origin: [i32; 3], filter: Identifier, width_dir: Direction, extra_width: i32, depth_dir: Direction, extra_depth: i32, block_at: &'a BlockAt<'a>, in_tag_at: &'a InTagAt<'a>) -> Self {
        let mut walk = Self {
            block_at, in_tag_at, filter,
            min_x: origin[0], max_x: origin[0], min_z: origin[2], max_z: origin[2],
            branch_visited: HashSet::new(), queue: VecDeque::new(),
        };
        walk.queue.push_back(TreePos { pos: add(origin, [0, 1, 0], 1), direction: Direction::Up, checked: false });
        if extra_depth > 0 || extra_width > 0 {
            for d in 0..=extra_depth {
                for w in -extra_width..=extra_width {
                    if d == 0 && w == 0 { continue; }
                    let pos = add(add(origin, vec_of(depth_dir), d), vec_of(width_dir), w);
                    if walk.is_valid(pos) {
                        walk.queue.push_back(TreePos { pos, direction: Direction::Up, checked: true });
                        walk.min_x = walk.min_x.min(pos[0]);
                        walk.max_x = walk.max_x.max(pos[0]);
                        walk.min_z = walk.min_z.min(pos[2]);
                        walk.max_z = walk.max_z.max(pos[2]);
                    }
                }
            }
        }
        walk
    }

    fn is_valid(&self, pos: [i32; 3]) -> bool { (self.block_at)(pos).as_ref() == Some(&self.filter) }

    fn outside_trunk(&self, pos: [i32; 3]) -> bool {
        pos[0] < self.min_x || pos[0] > self.max_x || pos[2] < self.min_z || pos[2] > self.max_z
    }

    /// isBranch: outside the trunk, within branch distance of the nearest trunk corner, and not yet
    /// visited as a branch (the visit is recorded either way); the block must match.
    fn is_branch(&mut self, pos: [i32; 3]) -> bool {
        if !self.outside_trunk(pos) { return false; }
        let delta_x = (pos[0] - self.min_x).abs().min((pos[0] - self.max_x).abs());
        let delta_z = (pos[2] - self.min_z).abs().min((pos[2] - self.max_z).abs());
        if delta_x + delta_z > 10 || self.branch_visited.contains(&pos) { return false; }
        self.branch_visited.insert(pos);
        self.is_valid(pos)
    }

    /// canOcclude of the block below, approximated (see the struct notes).
    fn occludes_below(&self, pos: [i32; 3]) -> bool {
        let below = add(pos, [0, -1, 0], 1);
        match (self.block_at)(below) {
            None => false,
            Some(block) => block != self.filter && !(self.in_tag_at)(below, &Identifier::parse("minecraft:leaves").expect("valid tag")),
        }
    }

    /// tryBranch: a branch candidate joins the queue only over a non-occluding block.
    fn try_branch(&mut self, pos: [i32; 3], direction: Direction) {
        if self.is_branch(pos) && !self.occludes_below(pos) {
            self.queue.push_back(TreePos { pos, direction, checked: true });
        }
    }

    fn run(mut self) -> Vec<[i32; 3]> {
        let mut out = Vec::new();
        while let Some(mut tree_pos) = self.queue.pop_front() {
            if tree_pos.direction == Direction::Up {
                let is_tree_up = tree_pos.checked || self.is_valid(tree_pos.pos);
                for direction in [Direction::North, Direction::South, Direction::West, Direction::East] {
                    let candidate = add(tree_pos.pos, vec_of(direction), 1);
                    self.try_branch(candidate, if is_tree_up { direction } else { Direction::Up });
                }
                if is_tree_up {
                    let (x, z) = (tree_pos.pos[0], tree_pos.pos[2]);
                    if x == self.min_x && z == self.min_z { let c = add(tree_pos.pos, [-1, 0, -1], 1); self.try_branch(c, Direction::West); }
                    if x == self.min_x && z == self.max_z { let c = add(tree_pos.pos, [-1, 0, 1], 1); self.try_branch(c, Direction::West); }
                    if x == self.max_x && z == self.min_z { let c = add(tree_pos.pos, [1, 0, -1], 1); self.try_branch(c, Direction::East); }
                    if x == self.max_x && z == self.max_z { let c = add(tree_pos.pos, [1, 0, 1], 1); self.try_branch(c, Direction::East); }
                    let returned = tree_pos.pos;
                    tree_pos.pos = add(tree_pos.pos, vec_of(tree_pos.direction), 1);
                    tree_pos.checked = false;
                    self.queue.push_back(tree_pos);
                    if self.outside_trunk(returned) { self.branch_visited.insert(returned); }
                    out.push(returned);
                }
            } else {
                let direction = tree_pos.direction;
                let pos = tree_pos.pos;
                // Continue in the branch's direction: up, then direction, then direction and up.
                let up = add(pos, [0, 1, 0], 1);
                if self.is_branch(up) {
                    self.queue.push_back(TreePos { pos: up, direction, checked: true });
                } else {
                    let along = add(up, vec_of(direction), 1);
                    let along = add(along, [0, -1, 0], 1);
                    if self.is_branch(along) {
                        self.queue.push_back(TreePos { pos: along, direction, checked: true });
                    } else if self.is_branch(add(along, [0, 1, 0], 1)) {
                        let pos = add(along, [0, 1, 0], 1);
                        self.queue.push_back(TreePos { pos, direction, checked: true });
                    }
                }
                // Then each side in turn: the side, side and up, side and direction, side, direction and up.
                for rotated in [clockwise(direction), counter_clockwise(direction)] {
                    let side = add(pos, vec_of(rotated), 1);
                    if self.is_branch(side) {
                        self.queue.push_back(TreePos { pos: side, direction: rotated, checked: true });
                    } else if self.is_branch(add(side, [0, 1, 0], 1)) {
                        let pos = add(side, [0, 1, 0], 1);
                        self.queue.push_back(TreePos { pos, direction: rotated, checked: true });
                    } else {
                        let along = add(add(side, vec_of(direction), 1), [0, -1, 0], 1);
                        if self.is_branch(along) {
                            self.queue.push_back(TreePos { pos: along, direction: rotated, checked: true });
                        } else if self.is_branch(add(along, [0, 1, 0], 1)) {
                            let pos = add(along, [0, 1, 0], 1);
                            self.queue.push_back(TreePos { pos, direction: rotated, checked: true });
                        }
                    }
                }
                out.push(pos);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn no_tags(_: [i32; 3], _: &Identifier) -> bool { false }

    #[test]
    fn a_pickaxe_box_grows_with_expanded_levels() {
        let fields = json!({"type": "tconstruct:box_aoe", "expansion_direction": "tconstruct:pitch", "expansions": [{"depth": 2}, {"height": 1}]});
        let iterator = Iterator::decode(fields.as_object().unwrap()).unwrap().unwrap();
        let air = |_: [i32; 3]| -> Option<Identifier> { None };
        // Level 0: the bonus alone, which is zero — nothing extra.
        assert!(iterator.positions([0, 64, 0], Direction::North, 180.0, 0.0, 0, &air, &no_tags).is_empty());
        // Level 1: the first expansion only (depth 2, facing north at pitch 0: depth runs north).
        let level1 = iterator.positions([0, 64, 0], Direction::North, 180.0, 0.0, 1, &air, &no_tags);
        assert_eq!(level1, vec![[0, 64, -1], [0, 64, -2]]);
        // Level 2: both expansions (depth 2 and height 1, traversed both ways).
        let level2 = iterator.positions([0, 64, 0], Direction::North, 180.0, 0.0, 2, &air, &no_tags);
        assert_eq!(level2.len(), 3 * 3 - 1, "a 1x3x3 plate minus the origin: {level2:?}");
        // Level 3: one full set plus the first expansion again (depth 4, height 1).
        let level3 = iterator.positions([0, 64, 0], Direction::North, 180.0, 0.0, 3, &air, &no_tags);
        assert_eq!(level3.len(), 3 * 5 - 1, "a 1x3x5 plate minus the origin: {level3:?}");
    }

    #[test]
    fn a_circle_of_one_does_nothing_and_three_makes_a_disc() {
        let fields = json!({"type": "tconstruct:circle_aoe", "diameter": 1});
        let iterator = Iterator::decode(fields.as_object().unwrap()).unwrap().unwrap();
        let air = |_: [i32; 3]| -> Option<Identifier> { None };
        assert!(iterator.positions([0, 64, 0], Direction::Up, 0.0, 0.0, 0, &air, &no_tags).is_empty());
        let fields = json!({"type": "tconstruct:circle_aoe", "diameter": 3});
        let iterator = Iterator::decode(fields.as_object().unwrap()).unwrap().unwrap();
        let disc = iterator.positions([0, 64, 0], Direction::Up, 0.0, 0.0, 0, &air, &no_tags);
        assert_eq!(disc.len(), 8, "the 3x3 disc around the origin: {disc:?}");
        assert!(disc.iter().all(|pos| distance_sq([0, 64, 0], *pos) <= 2));
    }

    #[test]
    fn a_vein_follows_only_the_same_block() {
        let fields = json!({"type": "tconstruct:vein_aoe", "max_distance": 2});
        let iterator = Iterator::decode(fields.as_object().unwrap()).unwrap().unwrap();
        let ore = id("minecraft:iron_ore");
        let blocks = |pos: [i32; 3]| -> Option<Identifier> {
            match pos { [0, 64, 0] | [1, 64, 0] | [2, 64, 0] | [3, 64, 0] | [0, 65, 0] => Some(ore.clone()), _ => None }
        };
        let vein = iterator.positions([0, 64, 0], Direction::North, 0.0, 0.0, 0, &blocks, &no_tags);
        assert_eq!(vein, vec![[0, 65, 0], [1, 64, 0], [2, 64, 0]], "two steps out, the third ore is too far: {vein:?}");
    }

    #[test]
    fn a_conditional_picks_its_branch_from_the_origin_block() {
        let fields = json!({"type": "tconstruct:conditional_aoe",
            "condition": {"type": "mantle:tag", "tag": "tconstruct:tree_log"},
            "if_true": {"type": "tconstruct:tree_aoe"},
            "if_false": {"type": "tconstruct:box_aoe", "expansion_direction": "tconstruct:height", "bonus": {"width": 1, "height": 1, "depth": 1}}});
        let iterator = Iterator::decode(fields.as_object().unwrap()).unwrap().unwrap();
        let log = id("minecraft:oak_log");
        // A four-log column; the tree branch fells the three above the origin.
        let blocks = |pos: [i32; 3]| -> Option<Identifier> {
            if pos[0] == 0 && pos[2] == 0 && (64..=67).contains(&pos[1]) { Some(log.clone()) } else { None }
        };
        let in_tag = |_: [i32; 3], tag: &Identifier| tag == &id("tconstruct:tree_log");
        let felled = iterator.positions([0, 64, 0], Direction::North, 180.0, 0.0, 0, &blocks, &in_tag);
        assert_eq!(felled, vec![[0, 65, 0], [0, 66, 0], [0, 67, 0]]);
        // A block outside the tag takes the box branch instead (height expansion: up only, both sides).
        let stone = id("minecraft:stone");
        let blocks = |pos: [i32; 3]| -> Option<Identifier> { if pos == [0, 64, 0] { Some(stone.clone()) } else { None } };
        let in_tag = |_: [i32; 3], _: &Identifier| false;
        let boxed = iterator.positions([0, 64, 0], Direction::North, 180.0, 0.0, 0, &blocks, &in_tag);
        assert_eq!(boxed.len(), 3 * 2 * 2 - 1, "the 3x2x2 box minus the origin: {boxed:?}");
        assert!(boxed.contains(&[0, 65, 1]), "up and away from the hit face");
        assert!(!boxed.contains(&[0, 64, 0]));
    }

    #[test]
    fn a_conditional_owes_the_default_predicate_only_through_a_box_or_circle_branch() {
        fn decode(fields: serde_json::Value) -> Iterator { Iterator::decode(fields.as_object().unwrap()).unwrap().unwrap() }
        let log = id("minecraft:oak_log");
        let blocks = |pos: [i32; 3]| -> Option<Identifier> { if pos == [0, 64, 0] { Some(log.clone()) } else { None } };
        let in_tag = |_: [i32; 3], tag: &Identifier| tag == &id("tconstruct:tree_log");
        let no_tag = |_: [i32; 3], _: &Identifier| false;
        // Condition met: the tree branch owes nothing. Condition unmet: the box branch owes it.
        let tree_or_box = decode(json!({"type": "tconstruct:conditional_aoe",
            "condition": {"type": "mantle:tag", "tag": "tconstruct:tree_log"},
            "if_true": {"type": "tconstruct:tree_aoe"},
            "if_false": {"type": "tconstruct:box_aoe", "expansion_direction": "tconstruct:height", "bonus": {"width": 1, "height": 1, "depth": 1}}}));
        assert!(!tree_or_box.owes_default_predicate([0, 64, 0], &blocks, &in_tag), "the tree branch applies its own block rules");
        assert!(tree_or_box.owes_default_predicate([0, 64, 0], &blocks, &no_tag), "the box branch owes the predicate");
        // A circle branch owes it too; a vein branch does not.
        let circle_or_vein = decode(json!({"type": "tconstruct:conditional_aoe",
            "condition": {"type": "mantle:tag", "tag": "tconstruct:tree_log"},
            "if_true": {"type": "tconstruct:circle_aoe", "diameter": 3},
            "if_false": {"type": "tconstruct:vein_aoe", "max_distance": 2}}));
        assert!(circle_or_vein.owes_default_predicate([0, 64, 0], &blocks, &in_tag));
        assert!(!circle_or_vein.owes_default_predicate([0, 64, 0], &blocks, &no_tag));
        // A nested conditional follows the branch it takes all the way down.
        let nested = decode(json!({"type": "tconstruct:conditional_aoe",
            "condition": {"type": "mantle:tag", "tag": "tconstruct:tree_log"},
            "if_true": {"type": "tconstruct:conditional_aoe",
                "condition": {"type": "mantle:tag", "tag": "tconstruct:tree_log"},
                "if_true": {"type": "tconstruct:box_aoe", "expansion_direction": "tconstruct:height", "bonus": {"width": 1, "height": 1, "depth": 1}},
                "if_false": {"type": "tconstruct:vein_aoe", "max_distance": 2}},
            "if_false": {"type": "tconstruct:vein_aoe", "max_distance": 2}}));
        assert!(nested.owes_default_predicate([0, 64, 0], &blocks, &in_tag), "conditional in conditional, a box at the leaf");
        assert!(!nested.owes_default_predicate([0, 64, 0], &blocks, &no_tag));
        // No block at the origin: nothing is selected, nothing is owed.
        let air = |_: [i32; 3]| -> Option<Identifier> { None };
        assert!(!tree_or_box.owes_default_predicate([0, 64, 0], &air, &in_tag));
        // The plain kinds, for the record.
        assert!(decode(json!({"type": "tconstruct:box_aoe", "expansion_direction": "tconstruct:height", "bonus": {"width": 1}})).owes_default_predicate([0, 64, 0], &blocks, &in_tag));
        assert!(!decode(json!({"type": "tconstruct:vein_aoe", "max_distance": 2})).owes_default_predicate([0, 64, 0], &blocks, &in_tag));
    }
}
