// Original Ender slime propagule and leaf interactions.
use super::*;
use horizon_core::random::RandomSource;
fn block<'a>(level: &'a GameLevel<'_>, state: BlockStateId) -> Option<&'a horizon_world::chunk::BlockState> { level.registry.state(state) }
fn sapling(level: &GameLevel<'_>, state: BlockStateId) -> bool { block(level,state).is_some_and(|b|b.name=="tconstruct:ender_slime_sapling") }
fn leaves(level: &GameLevel<'_>, state: BlockStateId) -> bool { block(level,state).is_some_and(|b|b.name=="tconstruct:ender_slime_leaves") }
fn below(pos: BlockPos) -> Option<BlockPos> {Some(BlockPos::new(pos.x,pos.y.checked_sub(1)?,pos.z))}
fn above(pos: BlockPos) -> Option<BlockPos> {Some(BlockPos::new(pos.x,pos.y.checked_add(1)?,pos.z))}
fn hanging(level: &GameLevel<'_>,state: BlockStateId) -> bool {block(level,state).and_then(|b|b.properties.get("hanging")).is_some_and(|v|v=="true")}
fn age(level: &GameLevel<'_>,state: BlockStateId) -> Option<u8> {block(level,state)?.properties.get("age")?.parse().ok()}
fn set_property(level: &mut GameLevel<'_>,ticks: &mut Ticks,pos: BlockPos,state: BlockStateId,key: &str,value: &str,flags: i32) {
    let Some(mut next)=block(level,state).cloned() else {return;};
    let Some(prop)=next.properties.get_mut(key) else {return;};*prop=value.into();
    let Ok(next)=level.registry.resolve(&next) else {return;};let b=level.behaviours;b.set_block(level,ticks,pos,next,flags);
}
fn growth_chunks_loaded(level: &GameLevel<'_>, pos: BlockPos) -> bool {
    super::tconstruct_slime_tree_shape::chunks_loaded(pos.x, pos.z, 24, |x, z| level.world.loaded(x, z))
}
fn advance(level: &mut GameLevel<'_>,ticks: &mut Ticks,pos: BlockPos,state: BlockStateId) {
    let stage=block(level,state).and_then(|b|b.properties.get("stage")).map(String::as_str);
    if stage==Some("0") {set_property(level,ticks,pos,state,"stage","1",4);return;}
    if stage!=Some("1") {return;}
    // Keep live feature writes inside already loaded chunks. This is a
    // conservative boundary and can defer growth near unloaded chunks.
    if !growth_chunks_loaded(level, pos) { return; }
    let tall=level.world.random.next_float()<0.85;
    let b=level.behaviours;
    let replacement=horizon_client::pluto_fluid_facts::replacement_after_removal(b, state);
    b.set_block(level,ticks,pos,replacement,4);
    if !horizon_world::level::tconstruct_ender_live_place::place(b,level,ticks,pos,tall) {
        b.set_block(level,ticks,pos,state,4);
    }
}
pub(super) fn random(level: &mut GameLevel<'_>,ticks: &mut Ticks,pos: BlockPos,state: BlockStateId,handled: &mut bool) {
    if !sapling(level,state) {return;}*handled=true;
    if hanging(level,state) {if let Some(n)=age(level,state).filter(|n|*n<4) {set_property(level,ticks,pos,state,"age",&(n+1).to_string(),2);}}
    else if matches!(level.world.random.next_int_bound(7),Ok(0)) {advance(level,ticks,pos,state);}
}
pub(super) fn survive(level: &GameLevel<'_>,pos: BlockPos,state: BlockStateId,result: &mut Option<bool>) {
    if !sapling(level,state) {return;}
    if hanging(level,state) {
        let supported=above(pos).map(|p|level.block_state(p)).and_then(|s|Some(level.behaviours.tag("tconstruct:slimy_leaves")?.contains(level.behaviours.block(s)))).unwrap_or(false);
        *result=Some(supported);return;
    }
    let ground=below(pos).map(|p|level.block_state(p)).and_then(|s|block(level,s)).map(|s|s.name.as_str()).unwrap_or("");
    let dirt=matches!(ground,"tconstruct:earth_slime_dirt"|"tconstruct:sky_slime_dirt"|"tconstruct:ichor_slime_dirt"|"tconstruct:ender_slime_dirt");
    let grass=ground.strip_prefix("tconstruct:").and_then(|s|s.strip_suffix("_slime_grass")).and_then(|s|s.split_once('_')).is_some_and(|(f,d)|["earth","sky","ichor","ender","blood"].contains(&f)&&["earth","sky","ichor","ender","vanilla"].contains(&d));
    *result=Some(dirt||grass);
}
pub(super) fn valid(level: &GameLevel<'_>,pos: BlockPos,state: BlockStateId,result: &mut Option<bool>) {
    if leaves(level,state) {*result=Some(below(pos).is_some_and(|p|level.behaviours.is_air(level.block_state(p))));}
    else if sapling(level,state) {
        let stage=block(level,state).and_then(|b|b.properties.get("stage")).map(String::as_str);
        *result = Some(super::tconstruct_slime_tree_shape::propagule_bonemeal_ready(
            hanging(level,state), age(level,state), stage, || growth_chunks_loaded(level,pos)));
    }
}
pub(super) fn success(level: &mut GameLevel<'_>,state: BlockStateId,result: &mut bool) {
    if leaves(level,state) {*result=true;}
    else if sapling(level,state) {*result=if hanging(level,state) {age(level,state).is_some_and(|n|n<4)} else {level.world.random.next_float()<0.45};}
}
pub(super) fn perform(level: &mut GameLevel<'_>,ticks: &mut Ticks,pos: BlockPos,state: BlockStateId) {
    if leaves(level,state) {
        let Some(target)=below(pos) else {return;};
        if !level.behaviours.is_air(level.block_state(target)) {return;}
        let Some(id)=level.registry.default_state("tconstruct:ender_slime_sapling") else {return;};
        let Some(mut value)=block(level,id).cloned() else {return;};
        let Some(prop)=value.properties.get_mut("hanging") else {return;};*prop="true".into();
        let Ok(id)=level.registry.resolve(&value) else {return;};let b=level.behaviours;b.set_block(level,ticks,target,id,2);
    } else if sapling(level,state) {
        if hanging(level,state) {if let Some(n)=age(level,state).filter(|n|*n<4) {set_property(level,ticks,pos,state,"age",&(n+1).to_string(),2);}}
        else {advance(level,ticks,pos,state);}
    }
}
