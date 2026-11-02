use super::*;

// Take crafting results through the existing menu engine and stage the source
// side/player/empty-side move before committing the side inventory.
pub(super) fn quick(menu: &mut horizon_client::menu::Menu, env: &horizon_client::menu::Env<'_>, side: &mut Vec<Option<ItemStack>>, kind: super::tconstruct_chest_rules::ChestKind, tags: &std::collections::HashSet<String>) -> std::result::Result<(),()> {
    use horizon_client::menu::Input;
    for _ in 0..64 {
        let Some(output)=menu.slot(0).cloned() else {break;};
        let max=env.items.get(&output.item).map_or(64,|info|info.max_stack).max(1);
        let inventory:Vec<_>=(0..36).map(|i|menu.player_item(i).cloned()).collect();
        let plan=super::tconstruct_chest_rules::plan_crafted_output(kind,side,&inventory,output.clone(),max,tags.contains(output.item.as_ref()),|inventory,stack|super::tconstruct_tx::source_quick_move_player(inventory,stack,max,max>1));
        let Some((next_side,next_inventory,leftover))=plan else {break;};
        let previous=menu.carried.take();
        if menu.click(env,0,0,Input::Pickup).is_err() {menu.carried=previous;return Err(());}
        let actual=menu.carried.take();menu.carried=previous;
        if !actual.as_ref().is_some_and(|actual|actual.count==output.count && actual.same_item_same_components(&output)) {return Err(());}
        for (i,stack) in next_inventory.into_iter().enumerate() {menu.set_player_item(i,stack);}
        if let Some(stack)=leftover {menu.dropped.push(stack);}
        *side=next_side;
    }
    Ok(())
}
