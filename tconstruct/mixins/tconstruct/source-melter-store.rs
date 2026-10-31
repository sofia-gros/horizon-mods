// Store machine metadata beside the live chest-like inventory. Machine block
// entities use custom IDs, so Horizon preserves their extra fields instead of
// applying the vanilla chest canonicalizer during save.
use horizon_world::nbt::{Compound, Tag};
use super::tconstruct_world::World;

type Pos = (i32, i32, i32);

fn set_metadata(sources: &mut Vec<(Pos, Tag)>, pos: Pos, key: &str, value: Tag) -> bool {
    if !matches!(key, "tconstruct:melter" | "tconstruct:heater" | "tconstruct:faucet" | "tconstruct:table" | "tconstruct:basin" | "tconstruct:smeltery") { return false; }
    if let Some((_, tag)) = sources.iter_mut().find(|(at, _)| *at == pos) {
        if tag.get("id") != Some(&Tag::String(key.into())) { return false; }
        let Tag::Compound(map) = tag else { return false; };
        map.insert(key.into(), value);
    } else {
        let mut map = Compound::new();
        map.insert("id".into(), Tag::String(key.into()));
        for (name, value) in [("x", pos.0), ("y", pos.1), ("z", pos.2)] { map.insert(name.into(), Tag::Int(value)); }
        map.insert(key.into(), value);
        sources.push((pos, Tag::Compound(map)));
    }
    true
}

/// Remove metadata belonging to a block entity that was actually removed.
pub(super) fn clear(world: &mut World, pos: Pos) {
    let Some(chunk) = world.chunks.get_mut(&(pos.0 >> 4, pos.2 >> 4)) else { return; };
    let chunk = std::sync::Arc::make_mut(chunk);
    let Some(saved) = chunk.saved.as_mut() else { return; };
    let saved = std::sync::Arc::make_mut(saved);
    let before = saved.block_entity_sources.len();
    saved.block_entity_sources.retain(|(at, _)| *at != pos);
    if saved.block_entity_sources.len() != before { world.modified.insert((pos.0 >> 4, pos.2 >> 4)); }
}

/// A replaced vanilla block can leave its old source tag in chunk extras.
/// Never replace an unknown or malformed tag: only a distinct known ID.
pub(super) fn clear_different(world: &mut World, pos: Pos, key: &str) {
    let old = world.chunks.get(&(pos.0 >> 4, pos.2 >> 4))
        .and_then(|chunk| chunk.saved.as_deref())
        .and_then(|saved| saved.block_entity_sources.iter().find(|(at, _)| *at == pos))
        .and_then(|(_, tag)| tag.get("id"));
    if matches!(old, Some(Tag::String(id)) if id.to_string_lossy() != key) { clear(world, pos); }
}

pub(super) fn read(world: &World, pos: Pos, key: &str) -> Option<Tag> {
    let chunk = world.chunks.get(&(pos.0 >> 4, pos.2 >> 4))?;
    let saved = chunk.saved.as_deref()?;
    saved.block_entity_sources.iter().find(|(at, _)| *at == pos)?.1.get(key).cloned()
}

pub(super) fn write(world: &mut World, pos: Pos, key: &str, value: Tag) -> bool {
    let Some(chunk) = world.chunks.get_mut(&(pos.0 >> 4, pos.2 >> 4)) else { return false; };
    let chunk = std::sync::Arc::make_mut(chunk);
    let saved = std::sync::Arc::make_mut(chunk.saved.get_or_insert_default());
    if !set_metadata(&mut saved.block_entity_sources, pos, key, value) { return false; }
    world.modified.insert((pos.0 >> 4, pos.2 >> 4));
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replacement_clears_only_stale_metadata_and_removal_preserves_neighbors() {
        use horizon_world::registry::BlockStateId;
        use std::sync::Arc;
        let mut world = World::new(BlockStateId(0), 0);
        world.chunks.insert((0, 0), Arc::new(horizon_client::chunk::Chunk::empty(0, 0)));
        let a = (1, 70, 1); let b = (2, 70, 1);
        assert!(write(&mut world, a, "tconstruct:melter", Tag::Int(7)));
        assert!(write(&mut world, b, "tconstruct:melter", Tag::Int(9)));
        clear_different(&mut world, a, "tconstruct:melter");
        assert_eq!(read(&world, a, "tconstruct:melter"), Some(Tag::Int(7)));
        clear_different(&mut world, a, "tconstruct:table");
        assert!(read(&world, a, "tconstruct:melter").is_none());
        assert!(write(&mut world, a, "tconstruct:table", Tag::Int(3)));
        clear(&mut world, a);
        assert!(read(&world, a, "tconstruct:table").is_none());
        assert_eq!(read(&world, b, "tconstruct:melter"), Some(Tag::Int(9)));
    }
    #[test]
    fn metadata_preserves_inventory_source_and_rejects_other_entities() {
        let pos = (1, 70, -1); let mut sources = Vec::new();
        assert!(set_metadata(&mut sources, pos, "tconstruct:melter", Tag::Int(7)));
        assert_eq!(sources[0].1.get("id"), Some(&Tag::String("tconstruct:melter".into())));
        assert!(set_metadata(&mut sources, pos, "tconstruct:melter", Tag::Int(8)));
        assert_eq!(sources.len(), 1); assert_eq!(sources[0].1.get("tconstruct:melter"), Some(&Tag::Int(8)));
        assert!(!set_metadata(&mut sources, pos, "other:field", Tag::Int(9)));
        sources[0].1 = Tag::compound([("id", Tag::String("minecraft:beacon".into()))]);
        assert!(!set_metadata(&mut sources, pos, "tconstruct:melter", Tag::Int(9)));
    }
}

/// Stage every metadata edit before touching any chunk, so a transfer cannot
/// drain the source when a destination chunk or entity cannot be updated.
pub(super) fn write_many(world: &mut World, changes: &[(Pos, &str, Tag)]) -> bool {
    let mut staged = std::collections::BTreeMap::new();
    for (pos, key, value) in changes {
        let chunk_pos = (pos.0 >> 4, pos.2 >> 4);
        if !staged.contains_key(&chunk_pos) {
            let Some(chunk) = world.chunks.get(&chunk_pos) else { return false; };
            let sources = chunk.saved.as_deref().map(|saved| saved.block_entity_sources.clone()).unwrap_or_default();
            staged.insert(chunk_pos, sources);
        }
        if !set_metadata(staged.get_mut(&chunk_pos).unwrap(), *pos, key, value.clone()) { return false; }
    }
    for (chunk_pos, sources) in staged {
        let Some(chunk) = world.chunks.get_mut(&chunk_pos) else { return false; };
        let chunk = std::sync::Arc::make_mut(chunk);
        let saved = std::sync::Arc::make_mut(chunk.saved.get_or_insert_default());
        saved.block_entity_sources = sources;
        world.modified.insert(chunk_pos);
    }
    true
}
