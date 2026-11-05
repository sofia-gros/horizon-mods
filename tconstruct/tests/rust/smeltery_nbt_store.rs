#![allow(dead_code)]
#[path = "../../mixins/tconstruct/source-melter-state.rs"]
mod tconstruct_source_melter_state;
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

fn casting_world(raw: horizon_world::nbt::Tag) -> (tconstruct_world::World, (i32,i32,i32)) {
    use horizon_world::nbt::Tag;use std::sync::Arc;
    let mut world=tconstruct_world::World::default();let pos=(1,70,1);
    let outer=Tag::compound([("id",Tag::String("tconstruct:table".into())),("tconstruct:table",raw)]);
    world.chunks.insert((0,0),Arc::new(tconstruct_world::Chunk{saved:Some(Arc::new(tconstruct_world::Saved{block_entity_sources:vec![(pos,outer)]}))}));
    (world,pos)
}
fn casting_state() -> tconstruct_source_melter_state::Casting {
    use horizon_items::items::ItemStack;
    let mut c=tconstruct_source_melter_state::Casting::new();c.input=Some(ItemStack::new("tconstruct:ingot_cast",1));
    assert!(c.prepare("x:recipe",90,2,ItemStack::new("minecraft:copper_ingot",1),false,false));
    assert!(c.tank.insert("tconstruct:molten_copper",90));c
}
#[test]
fn casting_cooling_and_completion_preserve_native_extensions() {
    use horizon_world::nbt::Tag;
    let state=casting_state();let mut raw=state.to_nbt();
    if let Tag::Compound(f)=&mut raw {f.insert("future".into(),Tag::Int(7));if let Some(Tag::Compound(t))=f.get_mut(&"tank".into()){t.insert("future_tank".into(),Tag::Int(8));}}
    let (mut world,pos)=casting_world(raw);
    for completed in [false,true] {
        let raw=tconstruct_source_melter_store::read(&world,pos,"tconstruct:table").unwrap();
        let mut state=tconstruct_source_melter_state::Casting::from_nbt(&raw).unwrap();assert_eq!(state.tick(true),completed);
        assert!(tconstruct_source_melter_store::write(&mut world,pos,"tconstruct:table",state.to_nbt()));
        let saved=tconstruct_source_melter_store::read(&world,pos,"tconstruct:table").unwrap();
        assert_eq!(saved.get("future"),Some(&Tag::Int(7)));assert_eq!(saved.get("tank").unwrap().get("future_tank"),Some(&Tag::Int(8)));
        let restored=tconstruct_source_melter_state::Casting::from_nbt(&saved).unwrap();
        assert_eq!(restored.output.is_some(),completed);
        if completed {assert!(saved.get("tank").unwrap().get("fluid").is_none());assert_eq!(restored.tank.capacity,0);}
    }
}
#[test]
fn unchanged_opaque_items_survive_ticks_but_consumption_aborts_batch() {
    use horizon_world::nbt::Tag;
    let mut state=casting_state();state.consumes_cast=true;let mut raw=state.to_nbt();
    if let Tag::Compound(f)=&mut raw {if let Some(Tag::Compound(item))=f.get_mut(&"input".into()){item.insert("future_item".into(),Tag::Int(9));}}
    let input=raw.get("input").unwrap().clone();let (mut world,pos)=casting_world(raw);
    let raw=tconstruct_source_melter_store::read(&world,pos,"tconstruct:table").unwrap();let mut state=tconstruct_source_melter_state::Casting::from_nbt(&raw).unwrap();
    assert!(!state.tick(true));assert!(tconstruct_source_melter_store::write(&mut world,pos,"tconstruct:table",state.to_nbt()));
    assert_eq!(tconstruct_source_melter_store::read(&world,pos,"tconstruct:table").unwrap().get("input"),Some(&input));
    let faucet=(2,70,1);assert!(tconstruct_source_melter_store::write(&mut world,faucet,"tconstruct:faucet",Tag::compound([("pending",Tag::Int(90))])));
    let before=world.chunks[&(0,0)].saved.as_ref().unwrap().block_entity_sources.clone();world.modified.clear();
    assert!(state.tick(true));
    assert!(!tconstruct_source_melter_store::write_many(&mut world,&[(faucet,"tconstruct:faucet",Tag::compound([("pending",Tag::Int(0))])),(pos,"tconstruct:table",state.to_nbt())]));
    assert_eq!(world.chunks[&(0,0)].saved.as_ref().unwrap().block_entity_sources,before);assert!(world.modified.is_empty());
}
#[test]
fn casting_rejects_ambiguous_aliases_and_lossy_ids_but_snapshot_replaces() {
    use horizon_world::nbt::{Tag,JavaString};
    for variant in 0..3 {
        let state=casting_state();let mut raw=state.to_nbt();
        if let Tag::Compound(f)=&mut raw {match variant {
            0=>{f.insert("Items".into(),Tag::List(vec![]));},
            1=>{f.insert("recipe".into(),Tag::String(JavaString(vec![0xd800])));},
            _=>{if let Some(Tag::Compound(t))=f.get_mut(&"tank".into()){t.insert("Amount".into(),Tag::Int(90));}},
        }}
        let (mut world,pos)=casting_world(raw.clone());
        assert!(!tconstruct_source_melter_store::write(&mut world,pos,"tconstruct:table",state.to_nbt()));
        assert_eq!(tconstruct_source_melter_store::read(&world,pos,"tconstruct:table"),Some(raw));
        assert!(tconstruct_source_melter_store::write_snapshot(&mut world,pos,"tconstruct:table",state.to_nbt()));
        assert_eq!(tconstruct_source_melter_store::read(&world,pos,"tconstruct:table"),Some(state.to_nbt()));
    }
}
#[test]
fn basin_writes_preserve_extensions_too() {
    use horizon_world::nbt::Tag;
    let state=casting_state();let mut raw=state.to_nbt();
    if let Tag::Compound(f)=&mut raw {f.insert("basin_extension".into(),Tag::Int(41));}
    let (mut world,_)=casting_world(state.to_nbt());let pos=(3,70,1);
    assert!(tconstruct_source_melter_store::write(&mut world,pos,"tconstruct:basin",raw));
    let mut state=state;assert!(!state.tick(true));
    assert!(tconstruct_source_melter_store::write(&mut world,pos,"tconstruct:basin",state.to_nbt()));
    assert_eq!(tconstruct_source_melter_store::read(&world,pos,"tconstruct:basin").unwrap().get("basin_extension"),Some(&Tag::Int(41)));
}
