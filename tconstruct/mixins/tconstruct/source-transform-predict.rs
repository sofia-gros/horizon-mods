// The packet was already sent by remote_use_block. Recognize the same source
// transforms so a successful main-hand use does not also use the off hand.
fn tconstruct_transform_predict(game:&mut crate::game::Game<'_>,stack:&horizon_items::items::ItemStack,hit:&crate::game::Hit)->std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break,Continue};
    let Some((template,kind))=crate::game::tconstruct_source_transform_logic::mode(stack.item.as_ref()) else {return Continue(());};
    let max=crate::game::tconstruct_source_transform_logic::maximum(stack);
    if max.is_none_or(|max|stack.damage>=max) || !game.may_build() {return Break(false);}
    let fake=horizon_items::items::ItemStack::new(template,1);
    let matches=|p:[i32;3]|crate::game::tconstruct_source_transform_logic::probe(game,&fake,kind,p,hit.face).is_some();
    if matches(hit.pos) {return Break(true);}
    let positions=match stack.item.as_ref() {
        "tconstruct:scythe"=>crate::game::tconstruct_source_transform_geometry::box_area(hit.pos,hit.face,game.yaw,2),
        "tconstruct:excavator"=>crate::game::tconstruct_source_transform_geometry::box_area(hit.pos,hit.face,game.yaw,0),
        "tconstruct:broad_axe"=>Some((1..=5).filter_map(|dy|hit.pos[1].checked_add(dy).map(|y|[hit.pos[0],y,hit.pos[2]])).collect()),
        _=>Some(Vec::new()),
    };
    if stack.item.as_ref()=="tconstruct:broad_axe" {
        let name=game.res.facts().get(game.world.block(hit.pos[0],hit.pos[1],hit.pos[2])).name.clone();
        if game.res.items().block_in_tag("tconstruct:source_effective/tconstruct/tree_log",&name) {
            let mut tree=crate::game::tconstruct_source_chain_geometry::Tree::new(hit.pos);
            let same=|p:[i32;3]|game.res.facts().get(game.world.block(p[0],p[1],p[2])).name==name;
            let occludes=|p:[i32;3]|game.res.facts().get(game.world.block(p[0],p[1],p[2])).can_occlude;
            while let Some(p)=tree.next(same,occludes) {if matches(p) {return Break(true);}}
            return Break(false);
        }
    }
    Break(positions.is_some_and(|p|p.into_iter().any(matches)))
}
