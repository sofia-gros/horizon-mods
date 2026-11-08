//! Reading and changing the blocks of a world (Minecraft: `World.getBlockState`, `World.setBlockState`).
//!
//! A [`World`] is a handle to one dimension. Like [`ServerPlayer`](super::player_api::ServerPlayer), it reaches the game only while a
//! callback that carries a world runs (a command, a projectile impact, a tick); called anywhere else it does nothing and says so.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;

use super::base::{Identifier, IdentifierError};
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

    /// Minecraft: `playSound`. Plays the sound event `sound` (such as `minecraft:block.grass.place`) for the players near `at`. False if the sound is
    /// unknown, or the call is made outside a callback that carries a world.
    pub fn play_sound(&self, sound: &str, at: [f64; 3], volume: f32, pitch: f32) -> bool {
        with_current(false, |ops| ops.play_sound(&self.dimension, sound, at, volume, pitch))
    }

    /// Minecraft: `setBlockState`. False if it could not be set (chunk not loaded, unknown block, outside a callback).
    pub fn set_block_state(&self, pos: BlockPos, state: &BlockState) -> bool {
        with_current(false, |ops| ops.set_block_state(&self.dimension, pos, &state.to_string()))
    }
}

/// An in-memory world for tests: every block set is remembered, every other position reads as air.
#[derive(Default, Debug)]
pub struct MemoryWorldOps {
    /// The blocks set so far, by `(dimension, position)`.
    pub blocks: BTreeMap<(String, BlockPos), String>,
    /// The sounds played so far: sound event and position.
    pub sounds: Vec<(String, [f64; 3])>,
}

impl WorldOps for MemoryWorldOps {
    fn block_state(&mut self, dimension: &str, pos: BlockPos) -> Option<String> {
        Some(self.blocks.get(&(dimension.to_owned(), pos)).cloned().unwrap_or_else(|| "minecraft:air".to_owned()))
    }

    fn set_block_state(&mut self, dimension: &str, pos: BlockPos, state: &str) -> bool {
        self.blocks.insert((dimension.to_owned(), pos), state.to_owned());
        true
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
