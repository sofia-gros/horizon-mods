//! Reading and changing the blocks of a world (Minecraft: `World.getBlockState`, `World.setBlockState`).
//!
//! A [`World`] is a handle to one dimension. Like [`ServerPlayer`](super::player_api::ServerPlayer), it reaches the game only while a
//! callback that carries a world runs (a command, a projectile impact, a tick); called anywhere else it does nothing and says so.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;

use super::base::{Identifier, IdentifierError, NbtCompound};
use super::block_entity_api_v1::BlockPos;

/// A block with its properties, as `/setblock` writes it: `minecraft:oak_stairs[facing=east,half=bottom]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockState {
    /// The block, such as `minecraft:glass`.
    pub block: Identifier,
    /// The state's properties by name; a property left out has the block's default.
    pub properties: BTreeMap<String, String>,
}

impl BlockState {
    /// The block with its default properties.
    pub fn new(block: Identifier) -> Self {
        Self { block, properties: BTreeMap::new() }
    }

    /// The same state with one property set.
    pub fn with(mut self, key: &str, value: &str) -> Self {
        self.properties.insert(key.to_owned(), value.to_owned());
        self
    }

    /// The value of a property, if the state names it.
    pub fn property(&self, key: &str) -> Option<&str> {
        self.properties.get(key).map(String::as_str)
    }

    /// Reads `minecraft:oak_stairs[facing=east,half=bottom]` (the brackets are optional).
    /// Whether this block is in the block tag (`minecraft:mineable/pickaxe`, `tconstruct:mineable/hand_axe`); false for an unknown tag and
    /// outside a `with_world` scope.
    pub fn is_in_tag(&self, tag: &str) -> bool {
        let block = self.block.to_string();
        with_current(false, |ops| ops.block_in_tag(&block, tag))
    }

    pub fn parse(text: &str) -> Result<BlockState, IdentifierError> {
        let (name, rest) = match text.split_once('[') {
            Some((name, rest)) => (name, rest.trim_end_matches(']')),
            None => (text, ""),
        };
        let mut state = BlockState::new(Identifier::parse(name)?);
        for pair in rest.split(',').filter(|p| !p.is_empty()) {
            if let Some((key, value)) = pair.split_once('=') {
                state.properties.insert(key.trim().to_owned(), value.trim().to_owned());
            }
        }
        Ok(state)
    }
}

impl fmt::Display for BlockState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.block)?;
        if !self.properties.is_empty() {
            let joined: Vec<String> = self.properties.iter().map(|(k, v)| format!("{k}={v}")).collect();
            write!(f, "[{}]", joined.join(","))?;
        }
        Ok(())
    }
}

/// What the game can do for a world. PlutoLib implements it over Horizon's `Host`; a test can implement it (or use [`MemoryWorldOps`]).
pub trait WorldOps {
    /// The block state at `pos` as text, or None if the chunk is not loaded.
    fn block_state(&mut self, dimension: &str, pos: BlockPos) -> Option<String>;
    /// Sets the block at `pos` to the state given as text; false if it could not be set.
    fn set_block_state(&mut self, dimension: &str, pos: BlockPos, state: &str) -> bool;
    /// Plays the sound event `sound` for the players near `at` in the dimension; false if the sound is unknown or could not be played.
    fn play_sound(&mut self, dimension: &str, sound: &str, at: [f64; 3], volume: f32, pitch: f32) -> bool;
    /// Draw from the authoritative dimension's existing random stream, in [0, 1).
    /// None when the host cannot supply that level; never substitute a fresh RNG.
    fn random_float(&mut self, _dimension: &str) -> Option<f32> {
        None
    }
    /// Whether the block (`minecraft:stone`) is in the block tag (`minecraft:mineable/pickaxe`); false for an unknown tag.
    fn block_in_tag(&mut self, _block: &str, _tag: &str) -> bool {
        false
    }
    /// Asks the game to tick `block` at `pos` after `delay` game ticks (Minecraft: `scheduleTick`); false when the host cannot schedule from
    /// this callback, which is the default.
    fn schedule_block_tick(&mut self, _dimension: &str, _pos: BlockPos, _block: &str, _delay: i32) -> bool {
        false
    }
    /// The saved data of the block entity at `pos` (without its position and id); None where there is none or it is not loaded.
    fn block_entity_data(&mut self, _dimension: &str, _pos: BlockPos) -> Option<NbtCompound> {
        None
    }
    /// Merges `data` into the saved data of the block entity at `pos`: each key given replaces the same key, the others stay; false when there is
    /// no block entity there or the host cannot.
    fn merge_block_entity_data(&mut self, _dimension: &str, _pos: BlockPos, _data: &NbtCompound) -> bool {
        false
    }
}

thread_local! {
    static OPS: Cell<Option<*mut dyn WorldOps>> = const { Cell::new(None) };
}

struct Scope<'a> {
    previous: Option<*mut dyn WorldOps>,
    _borrow: PhantomData<&'a mut dyn WorldOps>,
}

impl Drop for Scope<'_> {
    fn drop(&mut self) {
        OPS.with(|slot| slot.set(self.previous));
    }
}

/// Runs `f` with `ops` available to every [`World`] method called (on this thread) during `f`.
pub fn with_world<R>(ops: &mut dyn WorldOps, f: impl FnOnce() -> R) -> R {
    // SAFETY: the pointer is only dereferenced while `f` runs, strictly inside the lifetime of `ops`; `Scope` clears it on every exit.
    let raw: *mut dyn WorldOps = unsafe { std::mem::transmute::<&mut dyn WorldOps, *mut dyn WorldOps>(ops) };
    let previous = OPS.with(|slot| slot.replace(Some(raw)));
    let _scope = Scope { previous, _borrow: PhantomData };
    f()
}

fn with_current<R>(default: R, f: impl FnOnce(&mut dyn WorldOps) -> R) -> R {
    match OPS.with(Cell::get) {
        // SAFETY: see `with_world`: the pointer is valid for as long as it is stored.
        Some(raw) => f(unsafe { &mut *raw }),
        None => default,
    }
}

thread_local! {
    static TICKING: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Runs `f` with `dimension` as the dimension of the block entity being ticked, for [`World::ticking`]. The host opens it around
/// every block entity ticker, together with the [`with_world`] scope.
pub fn with_ticking_dimension<R>(dimension: &str, f: impl FnOnce() -> R) -> R {
    let previous = TICKING.with(|slot| slot.replace(Some(dimension.to_owned())));
    let result = f();
    TICKING.with(|slot| *slot.borrow_mut() = previous);
    result
}

/// A box of blocks read whole by [`World::read_region`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    from: BlockPos,
    to: BlockPos,
    states: BTreeMap<BlockPos, BlockState>,
}

impl Region {
    /// The corners the region was read between, as given.
    pub fn bounds(&self) -> (BlockPos, BlockPos) { (self.from, self.to) }

    /// The state read at `pos`; `None` outside the region.
    pub fn state(&self, pos: BlockPos) -> Option<&BlockState> { self.states.get(&pos) }
}

/// A handle to one dimension's blocks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    /// The dimension id, such as `minecraft:overworld`.
    pub dimension: String,
}

impl World {
    /// A handle to the dimension `dimension`.
    pub fn new(dimension: &str) -> Self {
        Self { dimension: dimension.to_owned() }
    }

    /// Minecraft: `getBlockState`. None if the chunk is not loaded or the call is made outside a callback that carries a world.
    pub fn get_block_state(&self, pos: BlockPos) -> Option<BlockState> {
        let text = with_current(None, |ops| ops.block_state(&self.dimension, pos))?;
        BlockState::parse(&text).ok()
    }

    /// The world of the block entity being ticked (`BlockEntity.getLevel`); `None` outside a ticker the host opened the scope for.
    pub fn ticking() -> Option<World> {
        TICKING.with(|slot| slot.borrow().as_ref().map(|dimension| World::new(dimension)))
    }

    /// Minecraft: `Level.hasChunksAt(from, to)` followed by reading every block of the box. All or nothing: `None` when any block of
    /// the box is in a chunk that is not loaded, so a structure check never judges half a region. Corners may be given in any order.
    pub fn read_region(&self, from: BlockPos, to: BlockPos) -> Option<Region> {
        let min = [from[0].min(to[0]), from[1].min(to[1]), from[2].min(to[2])];
        let max = [from[0].max(to[0]), from[1].max(to[1]), from[2].max(to[2])];
        let mut states = BTreeMap::new();
        for x in min[0]..=max[0] {
            for y in min[1]..=max[1] {
                for z in min[2]..=max[2] {
                    states.insert([x, y, z], self.get_block_state([x, y, z])?);
                }
            }
        }
        Some(Region { from, to, states })
    }

    /// Minecraft: `playSound`. Plays the sound event `sound` (such as `minecraft:block.grass.place`) for the players near `at`. False if the sound is
    /// unknown, or the call is made outside a callback that carries a world.
    pub fn play_sound(&self, sound: &str, at: [f64; 3], volume: f32, pitch: f32) -> bool {
        with_current(false, |ops| ops.play_sound(&self.dimension, sound, at, volume, pitch))
    }

    /// Minecraft: `Level.random.nextFloat`, using the existing server-level
    /// stream. None outside a world callback, on a client, or for an unopened
    /// dimension. A draw advances the same stream later native operations use.
    pub fn random_float(&self) -> Option<f32> {
        with_current(None, |ops| ops.random_float(&self.dimension))
    }

    /// Minecraft: `setBlockState`. False if it could not be set (chunk not loaded, unknown block, outside a callback).
    pub fn set_block_state(&self, pos: BlockPos, state: &BlockState) -> bool {
        with_current(false, |ops| ops.set_block_state(&self.dimension, pos, &state.to_string()))
    }

    /// Minecraft: `scheduleTick`. Asks for the block at `pos` to be ticked after `delay` game ticks (its behaviour's `scheduled_tick` runs). False
    /// when the host cannot schedule from this callback, outside a callback, or for a delay below 1; the caller then relies on the block's own
    /// random ticks and neighbour updates.
    pub fn schedule_tick(&self, pos: BlockPos, block: &str, delay: i32) -> bool {
        delay > 0 && with_current(false, |ops| ops.schedule_block_tick(&self.dimension, pos, block, delay))
    }

    /// The saved data of the block entity at `pos` (Minecraft: `BlockEntity.saveWithoutMetadata`); None where there is none, the chunk is not loaded
    /// or the call is made outside a callback.
    pub fn block_entity_data(&self, pos: BlockPos) -> Option<NbtCompound> {
        with_current(None, |ops| ops.block_entity_data(&self.dimension, pos))
    }

    /// Merges `data` into the saved data of the block entity at `pos` (a controller telling the blocks of its structure who their master is). False
    /// if there is no block entity there, or outside a callback.
    pub fn merge_block_entity_data(&self, pos: BlockPos, data: &NbtCompound) -> bool {
        with_current(false, |ops| ops.merge_block_entity_data(&self.dimension, pos, data))
    }
}

/// An in-memory world for tests: every block set is remembered, every other position reads as air.
#[derive(Default, Debug)]
pub struct MemoryWorldOps {
    /// The blocks set so far, by `(dimension, position)`.
    pub blocks: BTreeMap<(String, BlockPos), String>,
    /// The sounds played so far: sound event and position.
    pub sounds: Vec<(String, [f64; 3])>,
    /// The `(block, tag)` pairs that are true for `block_in_tag`.
    pub tags: Vec<(String, String)>,
    /// The saved data of the block entities, by `(dimension, position)`.
    pub entities: BTreeMap<(String, BlockPos), NbtCompound>,
}

impl WorldOps for MemoryWorldOps {
    fn block_state(&mut self, dimension: &str, pos: BlockPos) -> Option<String> {
        Some(self.blocks.get(&(dimension.to_owned(), pos)).cloned().unwrap_or_else(|| "minecraft:air".to_owned()))
    }

    fn set_block_state(&mut self, dimension: &str, pos: BlockPos, state: &str) -> bool {
        self.blocks.insert((dimension.to_owned(), pos), state.to_owned());
        true
    }

    fn block_in_tag(&mut self, block: &str, tag: &str) -> bool {
        self.tags.iter().any(|(b, t)| b == block && t == tag)
    }

    fn block_entity_data(&mut self, dimension: &str, pos: BlockPos) -> Option<NbtCompound> {
        self.entities.get(&(dimension.to_owned(), pos)).cloned()
    }

    fn merge_block_entity_data(&mut self, dimension: &str, pos: BlockPos, data: &NbtCompound) -> bool {
        match self.entities.get_mut(&(dimension.to_owned(), pos)) {
            Some(held) => {
                held.extend(data.iter().map(|(key, value)| (key.clone(), value.clone())));
                true
            }
            None => false,
        }
    }

    fn play_sound(&mut self, _dimension: &str, sound: &str, at: [f64; 3], _volume: f32, _pitch: f32) -> bool {
        self.sounds.push((sound.to_owned(), at));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_draws_use_the_requested_level_stream_and_missing_hosts_do_not_invent_values() {
        struct RandomOps { draws: std::collections::VecDeque<f32>, queried: Vec<String> }
        impl WorldOps for RandomOps {
            fn block_state(&mut self, _: &str, _: BlockPos) -> Option<String> { None }
            fn set_block_state(&mut self, _: &str, _: BlockPos, _: &str) -> bool { false }
            fn play_sound(&mut self, _: &str, _: &str, _: [f64; 3], _: f32, _: f32) -> bool { false }
            fn random_float(&mut self, dimension: &str) -> Option<f32> {
                self.queried.push(dimension.to_owned());
                self.draws.pop_front()
            }
        }
        let world = World::new("minecraft:the_end");
        let mut ops = RandomOps { draws: [0.0, 0.75].into(), queried: vec![] };
        assert_eq!(world.random_float(), None);
        with_world(&mut ops, || {
            assert_eq!(world.random_float(), Some(0.0));
            assert_eq!(world.random_float(), Some(0.75));
            assert_eq!(world.random_float(), None);
        });
        assert_eq!(ops.queried, vec!["minecraft:the_end"; 3]);
        assert!(ops.draws.is_empty());
        assert_eq!(world.random_float(), None);
        with_world(&mut MemoryWorldOps::default(), || assert_eq!(world.random_float(), None));
    }

    #[test]
    fn a_region_is_read_whole_or_not_at_all_and_a_ticker_knows_its_world() {
        assert!(World::ticking().is_none());
        with_ticking_dimension("minecraft:the_nether", || assert_eq!(World::ticking().unwrap().dimension, "minecraft:the_nether"));
        assert!(World::ticking().is_none(), "the scope is closed again");
        let world = World::new("minecraft:overworld");
        assert!(world.read_region([0, 0, 0], [1, 1, 1]).is_none(), "outside a scope nothing is loaded");
        struct Edge;
        impl WorldOps for Edge {
            fn block_state(&mut self, _: &str, pos: BlockPos) -> Option<String> { (pos[0] < 2).then(|| "minecraft:stone".to_owned()) }
            fn set_block_state(&mut self, _: &str, _: BlockPos, _: &str) -> bool { false }
            fn play_sound(&mut self, _: &str, _: &str, _: [f64; 3], _: f32, _: f32) -> bool { false }
        }
        with_world(&mut Edge, || {
            let region = world.read_region([1, 0, 1], [0, 1, 0]).expect("loaded");
            assert_eq!(region.bounds(), ([1, 0, 1], [0, 1, 0]));
            assert_eq!(region.state([0, 1, 1]).unwrap().block.path(), "stone");
            assert!(region.state([5, 5, 5]).is_none());
            assert!(world.read_region([0, 0, 0], [2, 0, 0]).is_none(), "one block of the box is in a chunk that is not loaded");
        });
    }

    #[test]
    fn states_round_trip_through_text_and_a_world_reads_what_was_set_only_inside_a_scope() {
        let stairs = BlockState::parse("minecraft:oak_stairs[facing=east,half=bottom]").unwrap();
        assert_eq!(stairs.property("facing"), Some("east"));
        assert_eq!(stairs.to_string(), "minecraft:oak_stairs[facing=east,half=bottom]");
        assert_eq!(BlockState::parse("stone").unwrap().to_string(), "minecraft:stone");
        assert!(BlockState::parse("Bad Name").is_err());

        let world = World::new("minecraft:overworld");
        assert!(!world.set_block_state([0, 64, 0], &stairs), "outside a scope nothing is set");
        assert_eq!(world.get_block_state([0, 64, 0]), None);
        let mut ops = MemoryWorldOps::default();
        with_world(&mut ops, || {
            assert!(world.set_block_state([0, 64, 0], &stairs));
            assert_eq!(world.get_block_state([0, 64, 0]), Some(stairs.clone()));
            assert_eq!(world.get_block_state([1, 64, 0]).unwrap().to_string(), "minecraft:air");
        });
        assert_eq!(ops.blocks.len(), 1);
    }
}
