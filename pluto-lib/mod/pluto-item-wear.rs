// The game reports a player's item losing durability to mods only for a LAN player (lan_acting_uuid); the local player's
// wear skips that hook. Report it here with player id 0, which the API adapter resolves to the local player.
fn pluto_item_wear(game:&mut crate::game::Game<'_>,stack:&horizon_client::items::ItemStack,amount:u32)->std::ops::ControlFlow<u32> {
    if game.lan_acting_uuid.is_some() {return std::ops::ControlFlow::Continue(());}
    let component=|id:&str| stack.data().and_then(|data| data.other(id));
    if component("minecraft:unbreakable").is_some() || component("!minecraft:max_damage").is_some() || component("!minecraft:damage").is_some() {
        return std::ops::ControlFlow::Continue(());
    }
    let mut loss=game.durability_loss(stack,amount);
    if loss>0 {
        let mut points=loss as i32;
        let mut cancel=false;
        let _=crate::entry::pluto_item_wear_access::take();
        game.native_item_damaged(0,"",&stack.item,&mut points,&mut cancel);
        loss=if cancel {0} else {points.max(0) as u32};
        // A callback that rebuilt a worn armor piece (its damage and published attributes already follow the wear) replaces it.
        if let Some(rebuilt)=crate::entry::pluto_item_wear_access::take() {
            if let Some(index)=game.armor.iter().position(|worn| worn.as_ref().is_some_and(|piece| piece.item==stack.item && piece.damage==stack.damage && piece.count==stack.count)) {
                game.armor[index]=Some(rebuilt);
                loss=0;
            }
        }
    }
    if loss>0 {game.trigger_item_durability(stack,stack.damage+loss);}
    std::ops::ControlFlow::Break(loss)
}
