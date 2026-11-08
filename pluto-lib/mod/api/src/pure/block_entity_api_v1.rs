//! Block entities for a mod's blocks: data that belongs to a block position, saved with the chunk, ticked every game tick on the server
//! and optionally sent to clients (Minecraft: `BlockEntity`, `BlockEntityType`, `BlockEntityTicker`; Fabric:
//! `FabricBlockEntityTypeBuilder`).
//!
//! The block itself comes from the mod's content (its blocks and models). Here a block gets its block entity type: the data it starts
//! with, what it does each tick, and what clients are told of it.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::base::{Identifier, NbtCompound};

/// A block position `[x, y, z]`.
pub type BlockPos = [i32; 3];

/// What a ticker receives: where the block entity is and its saved data, which the ticker may change.
pub struct BlockEntityTick<'a> {
    /// The block entity's position.
    pub pos: BlockPos,
    /// The block entity's saved data, without the position and the id; changes are saved with the chunk.
    pub data: &'a mut NbtCompound,
}

type Ticker = Arc<dyn Fn(&mut BlockEntityTick<'_>) + Send + Sync>;
type UpdateTag = Arc<dyn Fn(&NbtCompound) -> NbtCompound + Send + Sync>;

/// The kind of block entity a block has (Minecraft: `BlockEntityType`).
#[derive(Clone)]
pub struct BlockEntityType {
    /// The block this type belongs to.
    pub block: Identifier,
    /// The type's own id, saved with the block entity; the block's id unless changed.
    pub id: Identifier,
    /// The data a new block entity starts with.
    pub initial_data: NbtCompound,
    ticker: Option<Ticker>,
    update_tag: Option<UpdateTag>,
}

impl BlockEntityType {
    /// A block entity type for `block`, with no data, no ticker and nothing sent to clients.
    pub fn new(block: Identifier) -> Self {
        Self { id: block.clone(), block, initial_data: NbtCompound::new(), ticker: None, update_tag: None }
    }

    /// The same type under another id.
    pub fn with_id(mut self, id: Identifier) -> Self {
        self.id = id;
        self
    }

    /// The same type, new block entities starting with `data`.
    pub fn initial_data(mut self, data: NbtCompound) -> Self {
        self.initial_data = data;
        self
    }

    /// The same type, ticked on the server every game tick by `ticker` (Fabric: `BlockEntityTicker`).
    pub fn ticker(mut self, ticker: impl Fn(&mut BlockEntityTick<'_>) + Send + Sync + 'static) -> Self {
        self.ticker = Some(Arc::new(ticker));
        self
    }

    /// The same type, clients being sent what `update_tag` makes of the saved data whenever a chunk holding it is sent or its block
    /// changes (Minecraft: `BlockEntity.getUpdateTag`).
    pub fn update_tag(mut self, update_tag: impl Fn(&NbtCompound) -> NbtCompound + Send + Sync + 'static) -> Self {
        self.update_tag = Some(Arc::new(update_tag));
        self
    }

    /// The same type, clients being sent all of its saved data.
    pub fn sync_data(self) -> Self {
        self.update_tag(NbtCompound::clone)
    }
}

static TYPES: Mutex<Vec<BlockEntityType>> = Mutex::new(Vec::new());

/// Registers block entity types (Fabric: `Registry.register(Registries.BLOCK_ENTITY_TYPE, ...)`).
pub struct BlockEntityTypeRegistry;

impl BlockEntityTypeRegistry {
    /// Gives `entity_type.block` its block entity. Registering the same block again replaces the earlier type.
    pub fn register(entity_type: BlockEntityType) {
        if let Ok(mut all) = TYPES.lock() {
            all.retain(|t| t.block != entity_type.block);
            all.push(entity_type);
        }
    }

    /// The type registered for `block` (an id such as `tconstruct:seared_melter`), if any.
    pub fn get(block: &str) -> Option<BlockEntityType> {
        TYPES.lock().ok()?.iter().find(|t| t.block.to_string() == block).cloned()
    }
}

fn by_entity_id(id: &str) -> Option<BlockEntityType> {
    TYPES.lock().ok()?.iter().find(|t| t.id.to_string() == id).cloned()
}

/// A new block entity for `block`: its type id and starting data; None if the block has no registered type.
pub(crate) fn create(block: &str) -> Option<(String, NbtCompound)> {
    BlockEntityTypeRegistry::get(block).map(|t| (t.id.to_string(), t.initial_data))
}

/// Whether the block's block entity ticks.
pub(crate) fn ticks(block: &str) -> bool {
    BlockEntityTypeRegistry::get(block).is_some_and(|t| t.ticker.is_some())
}

/// Runs the ticker of the block entity type `entity_id` on `data`; true if a ticker ran.
pub(crate) fn tick(entity_id: &str, pos: BlockPos, data: &mut NbtCompound) -> bool {
    match by_entity_id(entity_id).and_then(|t| t.ticker) {
        Some(ticker) => {
            ticker(&mut BlockEntityTick { pos, data });
            true
        }
        None => false,
    }
}

/// What clients are sent of the block entity type `entity_id` with `data`; None sends nothing.
pub(crate) fn update_tag(entity_id: &str, data: &NbtCompound) -> Option<NbtCompound> {
    by_entity_id(entity_id).and_then(|t| t.update_tag).map(|make| make(data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pure::base::{Nbt, NbtCompoundExt};

    fn id(text: &str) -> Identifier {
        Identifier::parse(text).unwrap()
    }

    #[test]
    fn a_registered_block_gets_its_data_a_ticker_and_the_update_tag_it_asked_for() {
        let mut start = NbtCompound::new();
        start.insert("heat".into(), Nbt::Int(0));
        BlockEntityTypeRegistry::register(
            BlockEntityType::new(id("test:furnace"))
                .with_id(id("test:furnace_entity"))
                .initial_data(start)
                .ticker(|tick| {
                    let heat = tick.data.get_int("heat", 0) + 1;
                    tick.data.insert("heat".into(), Nbt::Int(heat as i32));
                    tick.data.insert("x".into(), Nbt::Int(tick.pos[0]));
                })
                .update_tag(|data| {
                    let mut tag = NbtCompound::new();
                    tag.insert("heat".into(), data["heat"].clone());
                    tag
                }),
        );
        let (entity, mut data) = create("test:furnace").unwrap();
        assert_eq!(entity, "test:furnace_entity");
        assert!(ticks("test:furnace"));
        assert!(tick(&entity, [5, 64, 5], &mut data));
        assert!(tick(&entity, [5, 64, 5], &mut data));
        assert_eq!(data.get_int("heat", -1), 2);
        assert_eq!(data.get_int("x", -1), 5);
        let tag = update_tag(&entity, &data).unwrap();
        assert_eq!(tag.len(), 1, "only what the update tag chose is sent");
        assert_eq!(tag.get_int("heat", -1), 2);
        assert!(create("test:unknown").is_none());
        assert!(!ticks("test:unknown"));
        assert!(!tick("test:unknown_entity", [0; 3], &mut data));
    }

    #[test]
    fn a_type_without_a_ticker_does_not_tick_and_sync_data_sends_everything() {
        BlockEntityTypeRegistry::register(BlockEntityType::new(id("test:quiet")).sync_data());
        assert!(!ticks("test:quiet"));
        let mut data = NbtCompound::new();
        data.insert("a".into(), Nbt::Byte(1));
        assert_eq!(update_tag("test:quiet", &data), Some(data.clone()));
        assert!(!tick("test:quiet", [0; 3], &mut data));
        BlockEntityTypeRegistry::register(BlockEntityType::new(id("test:quiet")));
        assert!(update_tag("test:quiet", &data).is_none(), "registering again replaces the earlier type");
    }
}
