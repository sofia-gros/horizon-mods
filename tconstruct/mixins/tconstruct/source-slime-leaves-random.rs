fn tconstruct_slime_leaves_random(level: &mut GameLevel<'_>, state: BlockStateId, handled: &mut bool) {
    let Some(block) = level.registry.state(state) else { return; };
    if matches!(block.name.as_str(), "tconstruct:earth_slime_leaves" | "tconstruct:sky_slime_leaves" | "tconstruct:ender_slime_leaves")
        && level.behaviours.tag("tconstruct:slimy_logs").is_none() {
        *handled = true;
    }
}
