// Source slime-fungus soil and bonemeal rules, using the host huge-fungus writer.
use super::*;
use horizon_core::random::RandomSource;
fn kind(level: &GameLevel<'_>, state: BlockStateId) -> Option<&'static str> {
    match level.registry.state(state)?.name.as_str() {
        "tconstruct:blood_slime_sapling" => Some("blood"),
        "tconstruct:ichor_slime_sapling" => Some("ichor"),
        _ => None,
    }
}
fn ground(level: &GameLevel<'_>, pos: BlockPos) -> Option<BlockStateId> {
    Some(level.block_state(BlockPos::new(pos.x,pos.y.checked_sub(1)?,pos.z)))
}
fn allowed(level: &GameLevel<'_>, pos: BlockPos) -> bool {
    ground(level,pos).is_some_and(|state|level.behaviours.tag("tconstruct:slimy_soil").is_some_and(|tag|tag.contains(level.behaviours.block(state))))
}
pub(super) fn survive(level: &GameLevel<'_>,pos: BlockPos,state: BlockStateId,result: &mut Option<bool>) {
    if kind(level,state).is_some() {*result=Some(allowed(level,pos));}
}
pub(super) fn valid(level: &GameLevel<'_>,pos: BlockPos,state: BlockStateId,result: &mut Option<bool>) {
    if kind(level,state).is_some() {*result=Some(allowed(level,pos));}
}
pub(super) fn success(level: &mut GameLevel<'_>,state: BlockStateId,result: &mut bool) {
    if kind(level,state).is_some() {*result=level.world.random.next_float()<0.4;}
}
pub(super) fn perform(level: &mut GameLevel<'_>,ticks: &mut Ticks,pos: BlockPos,state: BlockStateId) {
    let Some(kind)=kind(level,state) else {return;};
    if !allowed(level,pos) {return;}
    let Some(ground)=ground(level,pos).and_then(|s|level.registry.state(s)) else {return;};
    let Some(short)=ground.name.strip_prefix("tconstruct:") else {return;};
    let feature=format!("tconstruct:slime_fungus/{kind}_{short}");
    // A huge cap reaches four blocks horizontally. Do not change unloaded chunks.
    for dx in [-5,5] {for dz in [-5,5] {
        let (Some(x),Some(z))=(pos.x.checked_add(dx),pos.z.checked_add(dz)) else {return;};
        if !level.world.loaded(x,z) {return;}
    }}
    let behaviours=level.behaviours;
    let _=behaviours.place_feature(level,ticks,&feature,pos);
}
