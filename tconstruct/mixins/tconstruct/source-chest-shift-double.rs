fn tconstruct_chest_shift_double(game: &mut crate::game::Game<'_>,right: bool) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break,Continue};
    if right || !game.has_shift_down() {return Continue(());}
    let direct=crate::game::tconstruct_chest_ui::kind(game).is_some();
    let builder=crate::game::tconstruct_ui::side_chest_kind(game).is_some();
    let crafting=crate::game::tconstruct_crafting_station::side(game).is_some();
    if !direct && !builder && !crafting {return Continue(());}
    let (slot,outside)=game.inventory_target();let Some(slot)=slot else {return Continue(());};if outside || slot==200 {return Continue(());}
    let now=std::time::Instant::now();
    if !game.last_inv_click.is_some_and(|(at,last,was_right)|last==slot && !was_right && now.duration_since(at).as_millis()<250) {return Continue(());}
    let Some(moved)=game.last_quick_moved.clone() else {return Continue(());};
    let _=game.menu_click(Some(slot),0,horizon_client::menu::Input::QuickMove);
    if game.remote.is_some() {
        game.send(horizon_protocol::packets::play::Serverbound::ContainerButtonClick(horizon_protocol::packets::play::ContainerButtonClick {container:game.remote_menu_id(),button:44000}));
    } else {
        let slots:Vec<usize>=if slot<36 {(0..36).collect()} else if direct {
            let (count,scroll)=crate::game::tconstruct_chest_ui::page(game).unwrap_or((0,0));let first=scroll*8;(first..(first+32).min(count)).map(|i|300+i).collect()
        } else if builder && (300..302).contains(&slot) {vec![300,301]}
        else if builder && (327..381).contains(&slot) {(327..381).collect()}
        else if crafting && (100..109).contains(&slot) {(100..109).collect()}
        else if crafting && (600..648).contains(&slot) {(600..648).collect()}
        else {Vec::new()};
        for index in slots {
            if game.slot_view(index).as_ref().is_some_and(|s|s.same_item_same_components(&moved)) {let _=game.menu_click(Some(index),0,horizon_client::menu::Input::QuickMove);}
        }
    }
    game.last_inv_click=None;game.inv_clicks.skip_release();
    Break(())
}
