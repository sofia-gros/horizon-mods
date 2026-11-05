// Store machine metadata beside the live chest-like inventory. Machine block
// entities use custom IDs, so Horizon preserves their extra fields instead of
// applying the vanilla chest canonicalizer during save.
use horizon_world::nbt::{Compound, Tag};
use super::tconstruct_world::World;

type Pos = (i32, i32, i32);

fn set_metadata(sources: &mut Vec<(Pos, Tag)>, pos: Pos, key: &str, value: Tag) -> bool {
    set_metadata_inner(sources, pos, key, value, true)
}
fn set_metadata_inner(sources: &mut Vec<(Pos, Tag)>, pos: Pos, key: &str, value: Tag, preserve_smeltery: bool) -> bool {
    if !matches!(key, "tconstruct:modifier_chest" | "tconstruct:anvil" | "tconstruct:melter" | "tconstruct:heater" | "tconstruct:faucet" | "tconstruct:table" | "tconstruct:basin" | "tconstruct:smeltery" | "tconstruct:fuel_tank" | "tconstruct:ingot_tank" | "tconstruct:lantern" | "tconstruct:casting_tank" | "tconstruct:fluid_cannon") { return false; }
    if let Some((_, tag)) = sources.iter_mut().find(|(at, _)| *at == pos) {
        if tag.get("id") != Some(&Tag::String(key.into())) { return false; }
        let Tag::Compound(map) = tag else { return false; };
        let value = if preserve_smeltery && key == "tconstruct:smeltery" {
            match map.get(&key.into()) {
                Some(previous) => match super::tconstruct_source_smeltery_nbt_preserve::merge_top(previous, &value) {
                    Some(merged) => merged,
                    None => return false,
                },
                None => value,
            }
        } else if preserve_smeltery && matches!(key, "tconstruct:table" | "tconstruct:basin") {
            match map.get(&key.into()) {
                Some(previous) => match super::tconstruct_source_melter_state::Casting::from_nbt(&value)
                    .and_then(|state| state.to_nbt_preserving(previous)) {
                    Some(merged) => merged,
                    None => return false,
                },
                None => value,
            }
        } else { value };
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
    if clear_metadata(&mut saved.block_entity_sources, pos) { world.modified.insert((pos.0 >> 4, pos.2 >> 4)); }
}

fn clear_metadata(sources: &mut Vec<(Pos, Tag)>, pos: Pos) -> bool {
    let before = sources.len();
    sources.retain(|(at, _)| *at != pos);
    sources.len() != before
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

/// Typed boundary for live casting interactions. Never encode opaque items
/// before checking them: decoding an encoded candidate cannot recover wire.
pub(super) fn write_casting(world: &mut World, pos: Pos, key: &str, state: &super::tconstruct_source_melter_state::Casting) -> bool {
    if !matches!(key, "tconstruct:table" | "tconstruct:basin") || !state.items_save_safe() { return false; }
    let Some(raw) = read(world, pos, key) else { return false; };
    let Some(next) = state.to_nbt_preserving(&raw) else { return false; };
    write(world, pos, key, next)
}

pub(super) fn read(world: &World, pos: Pos, key: &str) -> Option<Tag> {
    let chunk = world.chunks.get(&(pos.0 >> 4, pos.2 >> 4))?;
    let saved = chunk.saved.as_deref()?;
    let raw=&saved.block_entity_sources.iter().find(|(at, _)| *at == pos)?.1;
    if let Some(value)=raw.get(key) {return Some(value.clone());}
    if key=="tconstruct:modifier_chest" && matches!(raw.get("id"),Some(Tag::String(id)) if id.to_string_lossy()==key) {
        let mut data=Compound::new();if let Some(color)=raw.get("color") {data.insert("color".into(),color.clone());}
        return Some(Tag::Compound(data));
    }
    None
}

pub(super) fn write(world: &mut World, pos: Pos, key: &str, value: Tag) -> bool {
    let Some(chunk) = world.chunks.get_mut(&(pos.0 >> 4, pos.2 >> 4)) else { return false; };
    let chunk = std::sync::Arc::make_mut(chunk);
    let saved = std::sync::Arc::make_mut(chunk.saved.get_or_insert_default());
    if !set_metadata(&mut saved.block_entity_sources, pos, key, value) { return false; }
    world.modified.insert((pos.0 >> 4, pos.2 >> 4));
    true
}

/// Client snapshots replace the previous view rather than merging omitted fields.
pub(super) fn write_snapshot(world: &mut World, pos: Pos, key: &str, value: Tag) -> bool {
    let Some(chunk) = world.chunks.get_mut(&(pos.0 >> 4, pos.2 >> 4)) else { return false; };
    let chunk = std::sync::Arc::make_mut(chunk);
    let saved = std::sync::Arc::make_mut(chunk.saved.get_or_insert_default());
    if !set_metadata_inner(&mut saved.block_entity_sources, pos, key, value, false) { return false; }
    world.modified.insert((pos.0 >> 4, pos.2 >> 4));
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn smeltery_update_preserves_unknown_top_fields_and_rejects_malformed() {
        let pos = (3, 70, 1); let mut sources = Vec::new();
        let first = Tag::compound([("future", Tag::Int(42)), ("fuel", Tag::Long(20))]);
        let next = Tag::compound([("fuel", Tag::Long(19))]);
        assert!(set_metadata(&mut sources, pos, "tconstruct:smeltery", first));
        assert!(set_metadata(&mut sources, pos, "tconstruct:smeltery", next));
        let actual = sources[0].1.get("tconstruct:smeltery").unwrap();
        assert_eq!(actual.get("future"), Some(&Tag::Int(42)));
        assert_eq!(actual.get("fuel"), Some(&Tag::Long(19)));
        let before = sources.clone();
        assert!(!set_metadata(&mut sources, pos, "tconstruct:smeltery", Tag::Int(3)));
        assert_eq!(sources, before);
    }
    #[test]
    fn smeltery_lossy_tank_id_cannot_be_rewritten() {
        let pos = (3, 70, 1); let mut sources = Vec::new();
        let bad = Tag::String(horizon_world::nbt::JavaString(vec![0xd800]));
        let row = Tag::compound([("id", bad), ("amount", Tag::Long(1))]);
        let tank = Tag::compound([("capacity", Tag::Long(100)), ("fluids", Tag::List(vec![row]))]);
        let first = Tag::compound([("tank", tank.clone()), ("fuel", Tag::Long(10))]);
        assert!(set_metadata(&mut sources, pos, "tconstruct:smeltery", first));
        let same_tank = Tag::compound([("tank", tank), ("fuel", Tag::Long(9))]);
        assert!(set_metadata(&mut sources, pos, "tconstruct:smeltery", same_tank));
        let before = sources.clone();
        let changed = Tag::compound([("tank", Tag::compound([("capacity", Tag::Long(100)), ("fluids", Tag::List(vec![]))]))]);
        assert!(!set_metadata(&mut sources, pos, "tconstruct:smeltery", changed));
        assert_eq!(sources, before);
    }
    #[test]
    fn client_snapshot_drops_omitted_view_fields() {
        let pos = (3, 70, 1); let mut sources = Vec::new();
        let old = Tag::compound([("heat_start", Tag::Int(2)), ("heat_page", Tag::IntArray(vec![9])), ("slot_count", Tag::Int(10)), ("future", Tag::Int(42))]);
        let next = Tag::compound([("slot_count", Tag::Int(3))]);
        assert!(set_metadata(&mut sources, pos, "tconstruct:smeltery", old));
        assert!(set_metadata_inner(&mut sources, pos, "tconstruct:smeltery", next.clone(), false));
        assert_eq!(sources[0].1.get("tconstruct:smeltery"), Some(&next));
    }
    #[test]
    fn removal_preserves_neighbor_metadata_and_allows_replacement() {
        let mut sources = Vec::new();
        let a = (1, 70, 1); let b = (2, 70, 1);
        assert!(set_metadata(&mut sources, a, "tconstruct:melter", Tag::Int(7)));
        assert!(set_metadata(&mut sources, b, "tconstruct:melter", Tag::Int(9)));
        assert!(clear_metadata(&mut sources, a));
        assert!(!clear_metadata(&mut sources, a));
        assert!(set_metadata(&mut sources, a, "tconstruct:table", Tag::Int(3)));
        assert!(clear_metadata(&mut sources, a));
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].0, b);
        assert_eq!(sources[0].1.get("tconstruct:melter"), Some(&Tag::Int(9)));
    }
    #[test]
    fn anvil_alloy_and_inventory_metadata_share_one_source() {
        let pos = (3, 70, -2);
        let mut sources = Vec::new();
        let alloy = Tag::compound([("Metal", Tag::String("tconstruct:slimesteel_block".into()))]);
        assert!(set_metadata(&mut sources, pos, "tconstruct:anvil", alloy.clone()));
        assert_eq!(sources[0].1.get("id"), Some(&Tag::String("tconstruct:anvil".into())));
        assert_eq!(sources[0].1.get("tconstruct:anvil"), Some(&alloy));
        assert!(!set_metadata(&mut sources, pos, "tconstruct:melter", Tag::Int(1)));
        assert_eq!(sources[0].1.get("tconstruct:anvil"), Some(&alloy));
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
