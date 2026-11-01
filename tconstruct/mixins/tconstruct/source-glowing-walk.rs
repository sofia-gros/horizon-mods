// Source GlowWalker runs on a grounded, living wearer's block-position change.
fn tconstruct_glow_walk(game: &mut crate::game::Game<'_>, from: [f64;3], _was_on_ground: bool) {
    if game.remote.is_some() || game.spectator() { return; }
    let Some(player) = game.player.as_ref() else { return; };
    if !player.ground.on_ground || player.health.dead_or_dying() { return; }
    let at = [player.position.x, player.position.y, player.position.z];
    if from.map(|v| v.floor() as i32) == at.map(|v| v.floor() as i32) { return; }
    let Some(boots) = game.armor.get(3).and_then(Option::as_ref).cloned() else { return; };
    if crate::game::tconstruct_source_ranged_wear::is_broken(&boots) { return; }
    let traits = crate::game::tconstruct_source_material_traits::levels(&game.world, &boots);
    let level = crate::game::tconstruct_source_overslime::upgrade(&boots, "tconstruct:glowing")
        + traits.as_ref().and_then(|v| v.levels.get("tconstruct:glowing").copied()).unwrap_or(0);
    if level <= 0 { return; }
    let expanded = crate::game::tconstruct_source_overslime::upgrade(&boots, "tconstruct:expanded")
        + traits.as_ref().and_then(|v| v.levels.get("tconstruct:expanded").copied()).unwrap_or(0);
    for p in crate::game::tconstruct_glowing_placement::walk_candidates(at, level, expanded) {
        let [x,_,z] = p;
        if !game.world.loaded(x,z) || game.world.light(x,p[1],z) & 15 >= 3 { continue; }
        if !game.res.facts().is_air(game.world.block(x,p[1],z)) { continue; }
        match crate::game::tconstruct_glowing_world::place(game, p, [0,-1,0]) {
            Ok(true) => {
                if !game.creative {
                    let (loss, shield) = crate::game::tconstruct_source_overslime_host::damage_with_cause(game, &boots, 5, Some("tconstruct:glowing"));
                    if let Some(current) = game.armor.get_mut(3).and_then(Option::as_mut).filter(|s| s.item == boots.item) {
                        crate::game::tconstruct_source_overslime::spend(current, shield);
                        crate::game::tconstruct_tx::fixed_damage(current, loss);
                    }
                }
                let sound=game.res.facts().get(game.world.block(x,p[1],z)).sounds.place.clone();
                game.audio.play(&sound, Some([f64::from(x)+0.5,f64::from(p[1])+0.5,f64::from(z)+0.5]),1.0,1.0);
                return;
            }
            Ok(false) => {},
            Err(error) => { eprintln!("tconstruct Glowing walk: {error}"); return; },
        }
    }
}
