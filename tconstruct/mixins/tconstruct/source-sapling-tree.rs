// Planted Earth/Sky trees, using the same source shape as island generation.
use super::*;
use horizon_core::random::RandomSource;
use std::collections::BTreeSet;
use super::tconstruct_slime_tree_shape::{self as shape, Operation};

fn offset(p: [i32; 3], d: [i32; 3]) -> Option<[i32; 3]> { Some([p[0].checked_add(d[0])?,p[1].checked_add(d[1])?,p[2].checked_add(d[2])?]) }
fn at(p: [i32; 3]) -> BlockPos { BlockPos::new(p[0],p[1],p[2]) }
fn name<'a>(level: &'a GameLevel<'_>, state: BlockStateId) -> Option<&'a str> { Some(level.registry.state(state)?.name.as_str()) }
pub(super) fn sapling(level: &GameLevel<'_>, state: BlockStateId) -> bool {
    matches!(name(level,state), Some("tconstruct:earth_slime_sapling" | "tconstruct:sky_slime_sapling"))
}
fn in_tag(level: &GameLevel<'_>, state: BlockStateId, tag: &str) -> Option<bool> {
    Some(level.behaviours.tag(tag)?.contains(level.behaviours.block(state)))
}
fn replaceable(level: &GameLevel<'_>, pos: [i32; 3]) -> Option<bool> {
    let state=level.block_state(at(pos));
    Some(level.behaviours.is_air(state) || in_tag(level,state,"minecraft:leaves")?)
}
fn set_distance(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: [i32;3], distance: i32) -> Option<()> {
    let state=level.block_state(at(pos));
    let mut block=level.registry.state(state)?.clone();
    *block.properties.get_mut("distance")?=distance.to_string();
    let next=level.registry.resolve(&block).ok()?;
    let behaviours=level.behaviours; behaviours.set_block(level,ticks,at(pos),next,19);
    Some(())
}
const NEIGHBORS: [[i32;3];6]=[[0,-1,0],[0,1,0],[0,0,-1],[0,0,1],[-1,0,0],[1,0,0]];
fn distances(level: &mut GameLevel<'_>, ticks: &mut Ticks, logs: &BTreeSet<[i32;3]>) -> Option<()> {
    let mut current=BTreeSet::new();
    for log in logs { for d in NEIGHBORS {
        let Some(pos)=offset(*log,d) else {continue;};
        if logs.contains(&pos) {continue;}
        let state=level.block_state(at(pos));
        if level.registry.state(state)?.properties.contains_key("distance") {
            current.insert(pos); set_distance(level,ticks,pos,1)?;
        }
    }}
    for distance in 1..6 {
        let mut next=BTreeSet::new();
        for p in &current { for d in NEIGHBORS {
            let Some(q)=offset(*p,d) else {continue;};
            if current.contains(&q) || next.contains(&q) {continue;}
            let state=level.block_state(at(q));
            let Some(value)=level.registry.state(state)?.properties.get("distance").and_then(|v|v.parse::<i32>().ok()) else {continue;};
            if value>distance+1 {set_distance(level,ticks,q,distance+1)?;next.insert(q);}
        }}
        current=next;
    }
    Some(())
}
pub(super) fn growth_chunks_loaded(level: &GameLevel<'_>, pos: BlockPos) -> bool {
    shape::chunks_loaded(pos.x, pos.z, 10, |x, z| level.world.loaded(x, z))
}
fn grow(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, original: BlockStateId) -> Option<()> {
    let sky=name(level,original)? == "tconstruct:sky_slime_sapling";
    let (base, bound, trunk, leaves)=if sky {(5,4,"tconstruct:skyroot_log","tconstruct:sky_slime_leaves")} else {(4,3,"tconstruct:greenheart_log","tconstruct:earth_slime_leaves")};
    let draw=level.world.random.next_int_bound(bound).ok()? as u32;
    let double=if sky {Some(level.world.random.next_int_bound(10).ok()? as u32)} else {None};
    let height=shape::height(base,bound as u32,draw,double)?;
    let origin=[pos.x,pos.y,pos.z];
    let below=offset(origin,[0,-1,0])?;
    let soil=level.block_state(at(below));
    if !shape::valid_site(pos.y,height,level.world.bottom_y(),level.world.top_y(),in_tag(level,soil,"tconstruct:slimy_soil")?) {return None;}
    // The canopy and distance propagation can reach ten horizontal blocks.
    // Do not read or modify an unloaded chunk through the runtime adapter.
    if !growth_chunks_loaded(level, pos) { return None; }
    let trunk=level.registry.default_state(trunk)?;
    let leaves=level.registry.default_state(leaves)?;
    let air=level.registry.default_state("minecraft:air")?;
    in_tag(level,air,"minecraft:leaves")?;
    let dirt=in_tag(level,soil,"minecraft:dirt")?;
    let behaviours=level.behaviours;
    // AbstractTreeGrower removes the sapling before attempting the feature.
    behaviours.set_block(level,ticks,pos,air,4);
    if dirt {let state=level.registry.default_state("minecraft:dirt")?;behaviours.set_block(level,ticks,at(below),state,2);}
    let mut logs=BTreeSet::new();
    for dy in 0..height {
        let p=offset(origin,[0,dy,0])?;
        if replaceable(level,p)? {behaviours.set_block(level,ticks,at(p),trunk,2);logs.insert(p);}
    }
    let top=offset(origin,[0,height,0])?;
    for op in shape::canopy(false) {
        let (delta,next)=match op {Operation::Leaf(d)=>(d,leaves),Operation::Air(d)=>(d,air),Operation::Vine {..}=>return None};
        let p=offset(top,delta)?;
        if replaceable(level,p)? {behaviours.set_block(level,ticks,at(p),next,2);}
    }
    if logs.is_empty() {behaviours.set_block(level,ticks,pos,original,4);return None;}
    distances(level,ticks,&logs)?;
    Some(())
}
pub(super) fn advance(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId) {
    let Some(block)=level.registry.state(state) else {return;};
    let Some(stage)=block.properties.get("stage") else {return;};
    if stage=="0" {
        let mut next=block.clone();next.properties.insert("stage".into(),"1".into());
        if let Ok(next)=level.registry.resolve(&next) {let b=level.behaviours;b.set_block(level,ticks,pos,next,4);}
    } else if stage=="1" {let _=grow(level,ticks,pos,state);}
}
