fn tconstruct_chest_drag_view(game: &mut crate::game::Game<'_>) -> std::ops::ControlFlow<Option<DragView>> {
    use std::ops::ControlFlow::{Break,Continue};
    use crate::game::tconstruct_chest_rules as chest;
    let direct=crate::game::tconstruct_chest_ui::kind(game);
    let builder=crate::game::tconstruct_ui::side_chest_kind(game);
    let crafting=crate::game::tconstruct_crafting_station::side(game);
    let Some(kind)=direct.or(builder).or(crafting.map(|v|v.3)) else {return Continue(());};
    let Some((ty,slots))=game.inv_drag.clone() else {return Break(None);};
    if slots.len()<2 {return Break(Some(DragView {slots:slots.into_iter().map(|i|(i,None)).collect(),left:None}));}
    let Some(cursor)=game.carried.clone() else {return Break(None);};
    let max=game.res.items().get(&cursor.item).map_or(64,|info|info.max_stack).max(1);
    let tagged=game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v|v.as_ref().ok()).and_then(|t|t.get("tconstruct:chest_parts")).map_or(game.remote.is_some(),|v|v.contains(cursor.item.as_ref()));
    let pattern=game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v|v.as_ref().ok()).and_then(|t|t.get("tconstruct:patterns")).map_or(game.remote.is_some(),|v|v.contains(cursor.item.as_ref()));
    let mut side=vec![None;kind.slots()];
    let mut combined=if builder.is_some() {game.container.and_then(|pos|game.world.block_entities.get(&pos)).map(|e|e.slots()[..27.min(e.slots().len())].to_vec()).unwrap_or_else(||vec![None;27])} else if crafting.is_some() {game.craft_grid.clone()} else {Vec::new()};
    let prefix=combined.len();combined.extend(game.inventory.clone());
    let (offset,columns)=if direct.is_some() {let (_,offset)=crate::game::tconstruct_chest_ui::page(game).unwrap_or((0,0));(offset,8)} else if builder.is_some() {let offset=game.container.and_then(|p|game.world.chunks.__horizon_mixins_tconstruct_builder_side_scroll().get(&p)).map_or(0,|s|s.0);(offset,6)} else {let (_,count,offset,_)=crafting.unwrap();(offset,crate::game::tconstruct_source_side_layout::columns(count))};
    if direct.is_some() {
        if let Some(entity)=game.container.and_then(|p|game.world.block_entities.get(&p)) {
            if game.remote.is_some() {for (i,stack) in entity.slots().iter().take(32).enumerate() {if let Some(cell)=side.get_mut(offset*8+i) {*cell=stack.clone();}}}
            else if entity.slots().len()==side.len() {side.clone_from_slice(entity.slots());}
        }
    } else if builder.is_some() {
        if game.remote.is_some() {if let Some(entity)=game.container.and_then(|p|game.world.block_entities.get(&p)) {for (i,stack) in entity.slots().iter().skip(27).take(48).enumerate() {if let Some(cell)=side.get_mut(offset*6+i) {*cell=stack.clone();}}}}
        else if let Some(pos)=game.container.and_then(|p|game.world.chunks.__horizon_mixins_tconstruct_builder_side().get(&p)).copied() {if let Some(entity)=game.world.block_entities.get(&pos) {if entity.slots().len()==side.len() {side.clone_from_slice(entity.slots());}}}
    } else if let Some((pos,_,_,_))=crafting {
        if game.remote.is_some() {if let Some((_,_,_,items))=game.world.chunks.__horizon_mixins_tconstruct_crafting_side_client().as_ref() {for (i,stack) in items.iter().enumerate() {if let Some(cell)=side.get_mut(offset*columns+i) {*cell=stack.clone();}}}}
        else if let Some(items)=crate::game::tconstruct_crafting_station::side_items(game,pos) {side=items;}
    }
    if builder.is_some() && prefix!=27 || crafting.is_some() && prefix!=9 {return Break(None);}
    let indexed:Vec<_>=slots.iter().filter_map(|&slot| {
        let target=
        if slot<36 {Some((false,prefix+slot))}
        else if direct.is_some() && (300..556).contains(&slot) {Some((true,if game.remote.is_some(){offset*8+slot-300}else{slot-300}))}
        else if builder.is_some() && slot==300 && pattern {Some((false,0))}
        else if builder.is_some() && slot==301 {Some((false,1))}
        else if builder.is_some() && (327..381).contains(&slot) {Some((true,offset*6+slot-327))}
        else if crafting.is_some() && (100..109).contains(&slot) {Some((false,slot-100))}
        else if crafting.is_some() && (600..648).contains(&slot) {Some((true,offset*columns+slot-600))}
        else {None};
        target.map(|target|(slot,target))
    }).collect();
    let selected:Vec<_>=indexed.iter().map(|(_,target)|*target).collect();
    let before_side=side.clone();let before_combined=combined.clone();let mut carried=Some(cursor);
    let _=chest::quick_craft(kind,&mut side,&mut combined,&mut carried,&selected,ty as u8,max,tagged,game.creative);
    let mut shown=std::collections::HashMap::new();
    for &(slot,(is_side,index)) in &indexed {
        let (before,after)=if is_side {(before_side.get(index),side.get(index))}else{(before_combined.get(index),combined.get(index))};
        let displayed=match (before,after) {(_,Some(Some(stack))) if before.and_then(|s|s.as_ref()).map_or(0,|s|s.count)!=stack.count => Some((stack.count,stack.count==if is_side {kind.slot_limit(max)}else{max})),_=>None};
        shown.insert(slot,displayed);
    }
    Break(Some(DragView {slots:shown,left:Some(carried.map_or(0,|stack|stack.count))}))
}
