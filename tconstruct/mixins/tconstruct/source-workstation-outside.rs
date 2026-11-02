fn tconstruct_workstation_outside(game: &crate::game::Game<'_>,batch: &horizon_client::hud::HudBatch) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break,Continue};
    let station=crate::game::tconstruct_ui::kind(game);
    let craft=crate::game::tconstruct_crafting_station::position(game).is_some() || game.remote_menu_title().map(|t|t.plain()).as_deref()==Some("TConstruct Crafting Station");
    let direct=crate::game::tconstruct_chest_ui::kind(game).is_some();
    if station.is_none() && !craft && !direct {return Continue(());}
    let (ox,oy)=game.inventory_origin(batch);let (x,y)=(game.cursor.0-ox,game.cursor.1-oy);
    if let Some(start)=crate::game::tconstruct_source_workstation_tabs::start(game) {
        let tabs=crate::game::tconstruct_source_workstation_tabs::from_game(game,start);
        if crate::game::tconstruct_source_workstation_tabs::hit(tabs.len(),station==Some(true),x,y).is_some() {return Break(false);}
    }
    let count=if craft {crate::game::tconstruct_crafting_station::side(game).map_or(0,|(_,count,_,_)|count)} else if station==Some(false) {crate::game::tconstruct_ui::side_count(game)} else {0};
    if count>0 {
        let left=crate::game::tconstruct_source_side_layout::left(count);
        let width=crate::game::tconstruct_source_side_layout::columns(count) as f32*18.0+24.0;
        let height=crate::game::tconstruct_source_side_layout::visible_rows(count) as f32*18.0+24.0;
        if (left..left+width).contains(&x) && (0.0..height).contains(&y) {return Break(false);}
    }
    Continue(())
}
