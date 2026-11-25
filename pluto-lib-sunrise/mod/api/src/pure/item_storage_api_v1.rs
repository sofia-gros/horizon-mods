//! The item storage of registered block entities, followed through forwarding blocks: what a host adapter answers a
//! neighbour's item query with (Fabric: `ItemStorage.SIDED` lookups; NeoForge: the `IItemHandler` capability), written over
//! the hooks of [`BlockEntityType`](super::block_entity_api_v1::BlockEntityType) and a
//! [`BlockEntityStore`](super::fluid_storage_api_v1::BlockEntityStore) that holds the saved data. A block with an
//! `item_forward` hook (a chute in a smeltery wall) answers with the storage of the block it points at, narrowed by its
//! filter; a block with `sided_access` opens only the slots its faces name; a block with an `item_transfer_rule` refuses
//! the transfers its rule refuses. Any other block answers from all of its slots, both ways — the host's own container
//! machinery serves those, so this adapter is only asked about blocks whose storage is their hooks
//! ([`item_served_from_data`](super::block_entity_api_v1::item_served_from_data)).

use super::{
    block_api_v1::Direction,
    block_entity_api_v1::{BlockEntityTypeRegistry, BlockPos, ItemFilter, ItemSlot, TransferMode},
    fluid_storage_api_v1::BlockEntityStore,
    item_behavior::{self, MaxStackSizeContext},
    neighbour_api_v1::NeighbourSlot,
};

/// Item queries over a [`BlockEntityStore`].
pub struct ItemStorage<'a> {
    store: &'a mut dyn BlockEntityStore,
}

/// Forwards followed before giving up (a forward cycle answers nothing).
const MAX_HOPS: usize = 4;

struct Resolved {
    pos: BlockPos,
    block: String,
    data: super::base::NbtCompound,
    items: Vec<Option<ItemSlot>>,
    filter: ItemFilter,
}

/// Whether two stacks are the same item with the same components (the counts may differ): what may merge into one slot.
fn same_item(left: &ItemSlot, right: &ItemSlot) -> bool {
    left.item == right.item && left.damage == right.damage && left.max_damage == right.max_damage
        && left.custom_data == right.custom_data && left.tool == right.tool
        && left.attribute_modifiers == right.attribute_modifiers && left.enchantments == right.enchantments
        && left.custom_model_data == right.custom_model_data
}

/// The most one slot of `stack` holds: the item behavior's per-stack answer, or 64 (the common item default; the item's own
/// content-declared maximum is the host's business, and a behavior is how a mod lowers it per stack).
fn slot_capacity(stack: &ItemSlot) -> u32 {
    item_behavior::max_stack_size(&MaxStackSizeContext { item: stack.item.to_string(), custom_data: stack.custom_data.clone() }).unwrap_or(64)
}

impl<'a> ItemStorage<'a> {
    /// Queries over `store`.
    pub fn new(store: &'a mut dyn BlockEntityStore) -> Self {
        Self { store }
    }

    /// Follows forwards from `pos` to the block entity that holds the storage, with the filter that applies on the way.
    fn resolve(&mut self, mut pos: BlockPos) -> Option<Resolved> {
        let mut filter = ItemFilter::Any;
        for _ in 0..=MAX_HOPS {
            let block = self.store.block_at(pos)?;
            let (data, items) = self.store.read(pos)?;
            match BlockEntityTypeRegistry::item_forward_of(&block, &data, &items) {
                Some(forward) => {
                    if filter == ItemFilter::Any {
                        filter = forward.filter;
                    }
                    pos = forward.target;
                }
                None if BlockEntityTypeRegistry::item_forwards(&block) => return None,
                None => return Some(Resolved { pos, block, data, items, filter }),
            }
        }
        None
    }

    /// The slots the block at `pos` shows `face`, as the forward's filter, the face's sided access and the transfer rule
    /// narrow them; `None` where there is no storage.
    pub fn slots(&mut self, pos: BlockPos, face: Direction) -> Option<Vec<NeighbourSlot>> {
        let found = self.resolve(pos)?;
        if found.filter == ItemFilter::Closed {
            return Some(Vec::new());
        }
        let sided = BlockEntityTypeRegistry::sided_access(&found.block, face);
        let mut slots = Vec::new();
        for (slot, stack) in found.items.iter().enumerate() {
            if let Some(access) = &sided {
                if !access.insert.contains(&slot) && !access.take.contains(&slot) {
                    continue;
                }
            }
            if let ItemFilter::Only(wanted) = &found.filter {
                if stack.as_ref().is_some_and(|held| held.item != *wanted) {
                    continue;
                }
            }
            let can_take = sided.as_ref().is_none_or(|access| access.take.contains(&slot))
                && stack.as_ref().is_none_or(|held| BlockEntityTypeRegistry::item_transfer_allowed(&found.block, found.pos, &found.data, &found.items, slot, Some(face), TransferMode::Take, held.clone()));
            slots.push(NeighbourSlot { slot, stack: stack.clone(), can_take });
        }
        Some(slots)
    }

    /// Takes up to `count` out of `slot` of the block at `pos` through `face`; `None` when nothing came out (the slot is
    /// empty, the face does not open it, the filter or the rule refuses, or there is no storage).
    pub fn extract(&mut self, pos: BlockPos, face: Direction, slot: usize, count: u32) -> Option<ItemSlot> {
        if count == 0 {
            return None;
        }
        let mut found = self.resolve(pos)?;
        if let ItemFilter::Closed = found.filter {
            return None;
        }
        if let Some(access) = BlockEntityTypeRegistry::sided_access(&found.block, face) {
            if !access.take.contains(&slot) {
                return None;
            }
        }
        let held = found.items.get(slot)?.clone()?;
        if let ItemFilter::Only(wanted) = &found.filter {
            if held.item != *wanted {
                return None;
            }
        }
        if !BlockEntityTypeRegistry::item_transfer_allowed(&found.block, found.pos, &found.data, &found.items, slot, Some(face), TransferMode::Take, held.clone()) {
            return None;
        }
        let taken = held.count.min(count);
        if taken == 0 {
            return None;
        }
        let mut out = held.clone();
        out.count = taken;
        let left = held.count - taken;
        found.items[slot] = (left > 0).then(|| {
            let mut rest = held;
            rest.count = left;
            rest
        });
        self.store.write(found.pos, found.data, found.items);
        Some(out)
    }

    /// Puts as much of `stack` into the block at `pos` through `face` as its slots take (merging into stacks of the same
    /// item first, then into empty slots, each up to the stack's per-slot maximum); the leftover that did not fit, or `None`
    /// when everything went in. The whole stack comes back when the face opens no slot to it, the filter refuses it, the
    /// rule refuses every slot, or there is no storage.
    pub fn insert(&mut self, pos: BlockPos, face: Direction, stack: ItemSlot) -> Option<ItemSlot> {
        if stack.count == 0 {
            return None;
        }
        let Some(mut found) = self.resolve(pos) else { return Some(stack) };
        match &found.filter {
            ItemFilter::Closed => return Some(stack),
            ItemFilter::Only(wanted) if *wanted != stack.item => return Some(stack),
            _ => {}
        }
        let sided = BlockEntityTypeRegistry::sided_access(&found.block, face);
        let capacity = slot_capacity(&stack);
        let mut left = stack.count;
        // The slots this face opens for inserts, in order, that the rule allows this stack into.
        let candidates: Vec<usize> = (0..found.items.len())
            .filter(|slot| sided.as_ref().is_none_or(|access| access.insert.contains(slot)))
            .filter(|slot| BlockEntityTypeRegistry::item_transfer_allowed(&found.block, found.pos, &found.data, &found.items, *slot, Some(face), TransferMode::Insert, stack.clone()))
            .collect();
        for &slot in &candidates {
            if left == 0 {
                break;
            }
            let Some(held) = &found.items[slot] else { continue };
            if !same_item(held, &stack) {
                continue;
            }
            let room = capacity.saturating_sub(held.count);
            let moved = room.min(left);
            if moved == 0 {
                continue;
            }
            let mut merged = held.clone();
            merged.count += moved;
            found.items[slot] = Some(merged);
            left -= moved;
        }
        for &slot in &candidates {
            if left == 0 {
                break;
            }
            if found.items[slot].is_some() {
                continue;
            }
            let moved = capacity.min(left);
            if moved == 0 {
                continue;
            }
            let mut placed = stack.clone();
            placed.count = moved;
            found.items[slot] = Some(placed);
            left -= moved;
        }
        if left == stack.count {
            return Some(stack);
        }
        self.store.write(found.pos, found.data, found.items);
        (left > 0).then(|| {
            let mut rest = stack;
            rest.count = left;
            rest
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pure::base::{Identifier, Nbt, NbtCompound};
    use crate::pure::block_entity_api_v1::{BlockEntityType, ItemForward};
    use crate::pure::fluid_storage_api_v1::MemoryBlockEntities;

    fn id(text: &str) -> Identifier {
        Identifier::parse(text).unwrap()
    }

    fn slot(item: &str, count: u32) -> ItemSlot {
        ItemSlot::new(id(item), count)
    }

    fn world_with(blocks: &[([i32; 3], &str, Vec<Option<ItemSlot>>)]) -> MemoryBlockEntities {
        let mut world = MemoryBlockEntities::default();
        for (pos, block, items) in blocks {
            world.blocks.insert(*pos, block.to_string());
            world.entities.insert(*pos, (NbtCompound::new(), items.clone()));
        }
        world
    }

    #[test]
    fn a_chute_answers_with_its_masters_inventory_and_moves_items_through_it() {
        BlockEntityTypeRegistry::register(BlockEntityType::new(id("test:controller")).item_slots(2));
        BlockEntityTypeRegistry::register(BlockEntityType::new(id("test:chute")).item_slots(0)
            .item_forward(|data, _| data.get("master").and_then(|master| master.as_list()).and_then(|master| {
                let mut at = master.iter().filter_map(|part| part.as_i64().map(|value| value as i32));
                Some(ItemForward { target: [at.next()?, at.next()?, at.next()?], filter: ItemFilter::Any })
            })));
        let mut master = NbtCompound::new();
        master.insert("master".into(), Nbt::List(vec![Nbt::Int(5), Nbt::Int(64), Nbt::Int(5)]));
        let mut world = world_with(&[([5, 64, 5], "test:controller", vec![Some(slot("minecraft:iron_ingot", 10)), None])]);
        world.entities.insert([0, 64, 0], (master, Vec::new()));
        world.blocks.insert([0, 64, 0], "test:chute".into());
        let mut storage = ItemStorage::new(&mut world);
        let slots = storage.slots([0, 64, 0], Direction::Down).expect("the chute forwards the query");
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].stack.as_ref().map(|held| held.count), Some(10));
        let taken = storage.extract([0, 64, 0], Direction::Down, 0, 4).expect("iron comes out of the controller");
        assert_eq!(taken.count, 4);
        let left = storage.insert([0, 64, 0], Direction::Up, slot("minecraft:iron_ingot", 100));
        assert!(left.is_none(), "everything fit: 58 merged onto the 6 left, 42 into the empty slot");
        let slots = storage.slots([0, 64, 0], Direction::Down).unwrap();
        assert_eq!(slots[0].stack.as_ref().map(|held| held.count), Some(64));
        assert_eq!(slots[1].stack.as_ref().map(|held| held.count), Some(42));
    }

    #[test]
    fn sided_access_and_the_rule_gate_what_moves_and_a_forward_cycle_answers_nothing() {
        BlockEntityTypeRegistry::register(BlockEntityType::new(id("test:table")).item_slots(2)
            .sided_access(|face| match face {
                Direction::Up => super::super::block_entity_api_v1::SidedAccess { insert: vec![0], take: vec![] },
                Direction::Down => super::super::block_entity_api_v1::SidedAccess { insert: vec![], take: vec![1] },
                _ => super::super::block_entity_api_v1::SidedAccess::default(),
            })
            .item_transfer_rule(|transfer| transfer.mode == TransferMode::Take || transfer.stack.item.to_string() != "minecraft:stick"));
        BlockEntityTypeRegistry::register(BlockEntityType::new(id("test:loop")).item_slots(1)
            .item_forward(|_, _| Some(ItemForward { target: [9, 9, 9], filter: ItemFilter::Any })));
        let mut world = world_with(&[
            ([0, 0, 0], "test:table", vec![None, Some(slot("minecraft:gold_ingot", 3))]),
            ([9, 9, 9], "test:loop", vec![Some(slot("minecraft:stone", 1))]),
        ]);
        let mut storage = ItemStorage::new(&mut world);
        // From above only the input slot shows, and it cannot be taken from.
        let slots = storage.slots([0, 0, 0], Direction::Up).unwrap();
        assert_eq!(slots.len(), 1);
        assert_eq!((slots[0].slot, slots[0].can_take), (0, false));
        assert!(storage.extract([0, 0, 0], Direction::Up, 1, 1).is_none(), "the output does not open upward");
        assert_eq!(storage.extract([0, 0, 0], Direction::Down, 1, 2).map(|taken| taken.count), Some(2));
        // The rule refuses sticks into the input; gold goes in.
        assert_eq!(storage.insert([0, 0, 0], Direction::Up, slot("minecraft:stick", 5)).map(|rest| rest.count), Some(5));
        assert!(storage.insert([0, 0, 0], Direction::Up, slot("minecraft:gold_ingot", 5)).is_none());
        // A block that forwards to itself has no storage.
        assert_eq!(storage.slots([9, 9, 9], Direction::Up), None);
        assert!(storage.extract([9, 9, 9], Direction::Up, 0, 1).is_none());
    }
}
