// The packet was already sent by remote_use_block. Recognize the same source
// transforms so a successful main-hand use does not also use the off hand.
fn tconstruct_transform_predict(game:&mut crate::game::Game<'_>,stack:&horizon_items::items::ItemStack,hit:&crate::game::Hit)->std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break,Continue};
    let Some((template,kind))=crate::game::tconstruct_source_transform_mode::mode(stack.item.as_ref()) else {return Continue(());};
    if game.keys.contains(&winit::keyboard::KeyCode::ShiftLeft) {return Break(false);}
    let max=crate::game::tconstruct_source_transform_logic::maximum(stack);
    if max.is_none_or(|max|stack.damage>=max) || !game.may_build() {return Break(false);}
    // The authoritative crop harvest consumes this interaction. Avoid also
    // sending an off-hand use while its server-side update is in flight.
    if matches!(stack.item.as_ref(), "tconstruct:kama" | "tconstruct:scythe")
        && crate::game::tconstruct_source_crop_harvest::is_crop(game, hit.pos) { return Break(true); }
    let fake=horizon_items::items::ItemStack::new(template,1);
    let probe=|p:[i32;3]|crate::game::tconstruct_source_transform_logic::probe(game,&fake,kind,p,hit.face).map(|(_,sound,_,_,particle,_)|(p,sound,particle));
    let mut feedback=probe(hit.pos);
    let positions=match stack.item.as_ref() {
        "tconstruct:scythe"=>crate::game::tconstruct_source_transform_geometry::box_area(hit.pos,hit.face,game.yaw,2),
        "tconstruct:excavator"=>crate::game::tconstruct_source_transform_geometry::box_area(hit.pos,hit.face,game.yaw,0),
        "tconstruct:broad_axe"=>Some((1..=5).filter_map(|dy|hit.pos[1].checked_add(dy).map(|y|[hit.pos[0],y,hit.pos[2]])).collect()),
        _=>Some(Vec::new()),
    };
    if feedback.is_none() && stack.item.as_ref()=="tconstruct:broad_axe" {
        let name=game.res.facts().get(game.world.block(hit.pos[0],hit.pos[1],hit.pos[2])).name.clone();
        if game.res.items().block_in_tag("tconstruct:source_effective/tconstruct/tree_log",&name) {
            let mut tree=crate::game::tconstruct_source_chain_geometry::Tree::new(hit.pos);
            let same=|p:[i32;3]|game.res.facts().get(game.world.block(p[0],p[1],p[2])).name==name;
            let occludes=|p:[i32;3]|game.res.facts().get(game.world.block(p[0],p[1],p[2])).can_occlude;
            while let Some(p)=tree.next(same,occludes) {if let Some(found)=probe(p) {feedback=Some(found);break;}}
            if feedback.is_none() {return Break(false);}
        }
    }
    if feedback.is_none() { feedback=positions.and_then(|p|p.into_iter().find_map(probe)); }
    if let Some((at,sound,particle))=feedback {
        if particle != "tconstruct:campfire_douse" { game.transform_particle(&particle,at); }
        if !sound.is_empty() { game.audio.play(&sound,Some([f64::from(at[0])+0.5,f64::from(at[1])+0.5,f64::from(at[2])+0.5]),1.0,1.0); }
        Break(true)
    } else {Break(false)}
}
