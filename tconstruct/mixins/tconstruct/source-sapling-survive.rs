fn tconstruct_sapling_survive(level: &GameLevel<'_>, pos: BlockPos, state: BlockStateId, survives: &mut Option<bool>) {
    if !super::tconstruct_sapling_tree::sapling(level,state) {return;}
    let Some(y)=pos.y.checked_sub(1) else {*survives=Some(false);return;};
    let ground=level.block_state(BlockPos::new(pos.x,y,pos.z));
    let Some(name)=level.registry.state(ground).map(|s|s.name.as_str()) else {*survives=Some(false);return;};
    let dirt=matches!(name,"tconstruct:earth_slime_dirt"|"tconstruct:sky_slime_dirt"|"tconstruct:ichor_slime_dirt"|"tconstruct:ender_slime_dirt");
    let grass=name.strip_prefix("tconstruct:").and_then(|v|v.strip_suffix("_slime_grass")).and_then(|v|v.split_once('_')).is_some_and(|(foliage,soil)| ["earth","sky","ichor","ender","blood"].contains(&foliage) && ["earth","sky","ichor","ender","vanilla"].contains(&soil));
    *survives=Some(dirt||grass);
}
