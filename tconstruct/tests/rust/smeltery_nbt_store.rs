#![allow(dead_code)]
#[path = "../../mixins/tconstruct/source-melter-state.rs"] mod tconstruct_source_melter_state;
#[path = "../../mixins/tconstruct/source-smeltery-tank.rs"] mod tconstruct_source_smeltery_tank;
#[path = "../../mixins/tconstruct/source-smeltery-fuel.rs"] mod tconstruct_source_smeltery_fuel;
#[path = "../../mixins/tconstruct/source-smeltery-state.rs"] mod tconstruct_source_smeltery_state;
#[path = "../../mixins/tconstruct/source-smeltery-layout.rs"] mod tconstruct_source_smeltery_layout;
#[path = "../../mixins/tconstruct/source-smeltery-page.rs"] mod tconstruct_source_smeltery_page;
#[path = "../../mixins/tconstruct/source-machine-destroy-guard.rs"] mod tconstruct_source_machine_destroy_guard;
extern crate self as horizon_client;
mod containers {
    pub enum BlockEntity { Chest(Vec<Option<horizon_items::items::ItemStack>>) }
}
mod tconstruct_world {
    use std::{collections::{HashMap, HashSet}, sync::Arc, ops::{Deref, DerefMut}};
    use horizon_world::nbt::Tag;
    type Pos = (i32,i32,i32);
    #[derive(Clone, Default)] pub struct Saved { pub block_entity_sources: Vec<(Pos,Tag)> }
    #[derive(Clone, Default)] pub struct Chunk { pub saved: Option<Arc<Saved>> }
    #[derive(Default)] pub struct Chunks {
        pub map: HashMap<(i32,i32), Arc<Chunk>>,
        pub pages: HashMap<Pos, (usize,usize)>,
        pub containers: HashMap<Pos, ([Option<horizon_items::items::ItemStack>;2], u8)>,
    }
    impl Deref for Chunks { type Target = HashMap<(i32,i32), Arc<Chunk>>; fn deref(&self) -> &Self::Target { &self.map } }
    impl DerefMut for Chunks { fn deref_mut(&mut self) -> &mut Self::Target { &mut self.map } }
    impl Chunks {
        pub fn __horizon_mixins_tconstruct_smeltery_ui_pages(&self) -> &HashMap<Pos, (usize,usize)> { &self.pages }
        pub fn __horizon_mixins_tconstruct_smeltery_local_containers(&self) -> &HashMap<Pos, ([Option<horizon_items::items::ItemStack>;2], u8)> { &self.containers }
    }
    #[derive(Default)] pub struct Entities(HashMap<Pos, crate::containers::BlockEntity>);
    impl Deref for Entities { type Target = HashMap<Pos, crate::containers::BlockEntity>; fn deref(&self) -> &Self::Target { &self.0 } }
    impl DerefMut for Entities { fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 } }
    impl Entities { pub fn get_or_insert_with(&mut self, pos: Pos, f: impl FnOnce() -> crate::containers::BlockEntity) -> &mut crate::containers::BlockEntity { self.0.entry(pos).or_insert_with(f) } }
    #[derive(Default)] pub struct World {
        pub chunks: Chunks,
        pub modified: HashSet<(i32,i32)>,
        pub block_entities: Entities,
    }
}


#[path = "../../mixins/tconstruct/source-smeltery-nbt-preserve.rs"] mod tconstruct_source_smeltery_nbt_preserve;
#[path = "../../mixins/tconstruct/source-melter-store.rs"] mod tconstruct_source_melter_store;
#[test]
fn write_many_rejects_wrong_destination_without_mutating_first_chunk() {
    use horizon_world::nbt::Tag;
    use std::sync::Arc;
    let mut world = tconstruct_world::World::default();
    let p = (1, 70, 1); let q = (17, 70, 1);
    let mut state = tconstruct_source_smeltery_state::Smeltery::new(); state.fuel = 20;
    let first = Tag::compound([("id", Tag::String("tconstruct:smeltery".into())),
        ("tconstruct:smeltery", state.to_nbt())]);
    let second = Tag::compound([("id", Tag::String("other:machine".into()))]);
    world.chunks.insert((0,0), Arc::new(tconstruct_world::Chunk { saved: Some(Arc::new(tconstruct_world::Saved { block_entity_sources: vec![(p, first.clone())] })) }));
    world.chunks.insert((1,0), Arc::new(tconstruct_world::Chunk { saved: Some(Arc::new(tconstruct_world::Saved { block_entity_sources: vec![(q, second.clone())] })) }));
    state.fuel = 19; let next = state.to_nbt();
    assert!(!tconstruct_source_melter_store::write_many(&mut world, &[(p,"tconstruct:smeltery",next.clone()),(q,"tconstruct:smeltery",next)]));
    assert_eq!(world.chunks[&(0,0)].saved.as_ref().unwrap().block_entity_sources[0].1, first);
    assert_eq!(world.chunks[&(1,0)].saved.as_ref().unwrap().block_entity_sources[0].1, second);
    assert!(world.modified.is_empty());
}
