// The game's own item-wear hook is heard only for a LAN player, and the local player's wear skips it. This reports every wear of a
// player's item to the mods (player id 0 for the local player when the game names none, which the API adapter resolves), lets a
// callback rebuild a worn armor piece through `pluto_item_wear_access`, and replaces the game's own wear step.
fn pluto_item_wear(game:&mut crate::game::Game<'_>,stack:&sunrise_client::items::ItemStack,amount:u32)->std::ops::ControlFlow<u32> {
    let component=|id:&str| stack.data().and_then(|data| data.other(id));
    if component("minecraft:unbreakable").is_some() || component("!minecraft:max_damage").is_some() || component("!minecraft:damage").is_some() {
        return std::ops::ControlFlow::Continue(());
    }
    let mut loss=game.durability_loss(stack,amount);
    if loss>0 {
        let mut points=loss as i32;
        let mut cancel=false;
        let (uuid,name)=(game.lan_acting_uuid.unwrap_or(0),game.lan_acting_name.clone().unwrap_or_default());
        let _=crate::pluto_item_wear_access::take();
        game.native_item_damaged(uuid,&name,&stack.item,&mut points,&mut cancel);
        loss=if cancel {0} else {points.max(0) as u32};
        // A callback that rebuilt a worn armor piece (its damage and published attributes already follow the wear) replaces it.
        if let Some(rebuilt)=crate::pluto_item_wear_access::take() {
            if let Some(index)=game.armor.iter().position(|worn| worn.as_ref().is_some_and(|piece| piece.item==stack.item && piece.damage==stack.damage && piece.count==stack.count)) {
                game.armor[index]=Some(rebuilt);
                loss=0;
            }
        }
    }
    if loss>0 {game.trigger_item_durability(stack,stack.damage+loss);}
    std::ops::ControlFlow::Break(loss)
}
