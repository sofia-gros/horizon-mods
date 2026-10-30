// Store machine metadata in the existing chest block entity's preserved NBT.
// Horizon merges unknown fields from block_entity_sources when saving a live
// chest, and the normal chest path continues to own the actual inventory.
use horizon_world::nbt::{Compound, Tag};
use super::tconstruct_world::World;

type Pos = (i32, i32, i32);

fn set_metadata(sources: &mut Vec<(Pos, Tag)>, pos: Pos, key: &str, value: Tag) -> bool {
    if !matches!(key, "tconstruct:melter" | "tconstruct:heater" | "tconstruct:faucet" | "tconstruct:casting") { return false; }
    if let Some((_, tag)) = sources.iter_mut().find(|(at, _)| *at == pos) {
        if tag.get("id") != Some(&Tag::String("minecraft:chest".into())) { return false; }
        let Tag::Compound(map) = tag else { return false; };
        map.insert(key.into(), value);
    } else {
        let mut map = Compound::new();
        map.insert("id".into(), Tag::String("minecraft:chest".into()));
        for (name, value) in [("x", pos.0), ("y", pos.1), ("z", pos.2)] { map.insert(name.into(), Tag::Int(value)); }
        map.insert(key.into(), value);
        sources.push((pos, Tag::Compound(map)));
    }
    true
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
    fn metadata_preserves_inventory_source_and_rejects_other_entities() {
        let pos = (1, 70, -1); let mut sources = Vec::new();
        assert!(set_metadata(&mut sources, pos, "tconstruct:melter", Tag::Int(7)));
        assert_eq!(sources[0].1.get("id"), Some(&Tag::String("minecraft:chest".into())));
        assert!(set_metadata(&mut sources, pos, "tconstruct:melter", Tag::Int(8)));
        assert_eq!(sources.len(), 1); assert_eq!(sources[0].1.get("tconstruct:melter"), Some(&Tag::Int(8)));
        assert!(!set_metadata(&mut sources, pos, "other:field", Tag::Int(9)));
        sources[0].1 = Tag::compound([("id", Tag::String("minecraft:beacon".into()))]);
        assert!(!set_metadata(&mut sources, pos, "tconstruct:melter", Tag::Int(9)));
    }
}
