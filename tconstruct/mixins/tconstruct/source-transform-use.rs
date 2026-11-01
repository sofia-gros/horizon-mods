// Use source tool traits and area rules with Horizon's existing block transformer.
fn tconstruct_transform_use(game:&mut crate::game::Game<'_>, stack:&horizon_items::items::ItemStack, pos:[i32;3], face:[i32;3]) -> std::ops::ControlFlow<Result<bool>> {
    use std::ops::ControlFlow::{Break,Continue};
    use horizon_world::nbt::Tag;
    let (template,kind)=match stack.item.as_ref() {
        "tconstruct:mattock" | "tconstruct:kama" | "tconstruct:scythe" => ("minecraft:wooden_hoe",0),
        "tconstruct:excavator" => ("minecraft:wooden_shovel",1),
        "tconstruct:hand_axe" | "tconstruct:broad_axe" => ("minecraft:wooden_axe",2),
        _ => return Continue(()),
    };
    let max=stack.data().and_then(|d|d.other("minecraft:custom_data")).and_then(|d|match d.get("tic_max_damage"){Some(Tag::Int(max)) if *max>0=>Some(*max as u32),_=>None});
    if max.is_none_or(|max|stack.damage>=max) || !game.may_build() { return Break(Ok(false)); }
    let item=stack.item.clone();
    let fake=horizon_items::items::ItemStack::new(template,1);
    let actor=game.lan_acting;
    let blocked=match actor {
        Some(conn)=>match game.world.chunks.__horizon_mixins_tconstruct_transform_guest_hits().get(&conn) {
            Some((target,blocked)) if *target==pos=>blocked.clone(),
            _=>return Break(Ok(false)),
        },
        None=>Default::default(),
    };
    let initial=game.world.block(pos[0],pos[1],pos[2]);
    let original=game.res.facts().get(initial).name.to_string();
    let tree=item.as_ref()=="tconstruct:broad_axe" && game.res.items().block_in_tag("tconstruct:source_effective/tconstruct/tree_log",&original);
    enum Area { List(std::vec::IntoIter<[i32;3]>), Tree(crate::game::tconstruct_source_chain_geometry::Tree) }
    let positions=match item.as_ref() {
        "tconstruct:scythe"=>crate::game::tconstruct_source_transform_geometry::box_area(pos,face,game.yaw,2),
        "tconstruct:excavator"=>crate::game::tconstruct_source_transform_geometry::box_area(pos,face,game.yaw,0),
        "tconstruct:broad_axe" if !tree=>Some((1..=5).filter_map(|dy|pos[1].checked_add(dy).map(|y|[pos[0],y,pos[2]])).collect()),
        _=>Some(Vec::new()),
    };
    let mut area=if tree { Area::Tree(crate::game::tconstruct_source_chain_geometry::Tree::new(pos)) } else { let Some(p)=positions else{return Break(Ok(false));};Area::List(p.into_iter()) };
    let result=(|| -> Result<bool> {
        let mut transformed=false;
        let mut next=Some(pos);
        loop {
            let Some(at)=next else {break;};
            let intact=game.held().is_some_and(|s|s.item==item && s.damage<max.unwrap());
            if !intact {break;}
            if !blocked.contains(&(at[0],at[2])) {
                if let Some((new,sound,loot,_damage,particle))=game.block_transform(&fake,at,face) {
                    game.trigger_block("minecraft:item_used_on_block",at,stack);
                    game.level_set(at,new)?;
                    game.transform_particle(&particle,at);
                    if loot.as_deref()==Some("minecraft:till/rooted_dirt") {
                        game.pop_resource([at[0]+face[0],at[1]+face[1],at[2]+face[2]],horizon_items::items::ItemStack::new("minecraft:hanging_roots",1));
                    }
                    game.audio.play(&sound,Some([f64::from(at[0])+0.5,f64::from(at[1])+0.5,f64::from(at[2])+0.5]),1.0,1.0);
                    game.stats.add("used",&item,1);
                    if !game.creative {
                        let held=game.held().cloned();
                        if let Some(held)=held {
                            let loss=game.durability_loss(&held,1);
                            if let Some(current)=game.held_mut().as_mut().filter(|current|current.item==item) {crate::game::tconstruct_tx::fixed_damage(current,loss);}
                        }
                    }
                    game.post_block_game_event("minecraft:block_change",at,game.sculk_actor(),Some(new));
                    transformed=true;
                }
            }
            let same=|p:[i32;3]|game.res.facts().get(game.world.block(p[0],p[1],p[2])).name.as_ref()==original.as_str();
            let occludes=|p:[i32;3]|game.res.facts().get(game.world.block(p[0],p[1],p[2])).can_occlude;
            next=match &mut area {Area::List(iter)=>iter.next(),Area::Tree(iter)=>iter.next(same,occludes)};
        }
        Ok(transformed)
    })();
    if let Some(conn)=actor {game.world.chunks.__horizon_mixins_tconstruct_transform_guest_hits_mut().remove(&conn);}
    Break(result)
}
