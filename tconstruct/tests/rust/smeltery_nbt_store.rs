#![allow(dead_code)]
mod tconstruct_world {
    use std::{collections::{HashMap, HashSet}, sync::Arc};
    use horizon_world::nbt::Tag;
    #[derive(Clone, Default)] pub struct Saved { pub block_entity_sources: Vec<((i32,i32,i32), Tag)> }
    #[derive(Clone, Default)] pub struct Chunk { pub saved: Option<Arc<Saved>> }
    #[derive(Default)] pub struct World { pub chunks: HashMap<(i32,i32), Arc<Chunk>>, pub modified: HashSet<(i32,i32)> }
}
#[path = "../../mixins/tconstruct/source-smeltery-nbt-preserve.rs"]
mod tconstruct_source_smeltery_nbt_preserve;
#[path = "../../mixins/tconstruct/source-melter-store.rs"]
mod tconstruct_source_melter_store;
#[test]
fn write_many_rejects_wrong_destination_without_mutating_first_chunk() {
    use horizon_world::nbt::Tag;
    use std::sync::Arc;
    let mut world = tconstruct_world::World::default();
    let p = (1, 70, 1); let q = (17, 70, 1);
    let first = Tag::compound([("id", Tag::String("tconstruct:smeltery".into())),
        ("tconstruct:smeltery", Tag::compound([("future", Tag::Int(42)), ("fuel", Tag::Long(20))]))]);
    let second = Tag::compound([("id", Tag::String("other:machine".into()))]);
    world.chunks.insert((0,0), Arc::new(tconstruct_world::Chunk { saved: Some(Arc::new(tconstruct_world::Saved { block_entity_sources: vec![(p, first.clone())] })) }));
    world.chunks.insert((1,0), Arc::new(tconstruct_world::Chunk { saved: Some(Arc::new(tconstruct_world::Saved { block_entity_sources: vec![(q, second.clone())] })) }));
    let next = Tag::compound([("fuel", Tag::Long(19))]);
    assert!(!tconstruct_source_melter_store::write_many(&mut world, &[(p,"tconstruct:smeltery",next.clone()),(q,"tconstruct:smeltery",next)]));
    assert_eq!(world.chunks[&(0,0)].saved.as_ref().unwrap().block_entity_sources[0].1, first);
    assert_eq!(world.chunks[&(1,0)].saved.as_ref().unwrap().block_entity_sources[0].1, second);
    assert!(world.modified.is_empty());
}
