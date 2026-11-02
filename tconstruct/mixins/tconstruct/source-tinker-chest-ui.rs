use super::*;
use super::tconstruct_chest_rules::{self as rules, ChestKind};

/// Direct local chest screen. Guest menus use their authoritative packet path.
pub(super) fn kind(game: &Game<'_>) -> Option<ChestKind> {
    if game.remote.is_some() {
        return match game.remote_menu_title().map(|title|title.plain()).as_deref() {
            Some("TConstruct Part Chest") => Some(ChestKind::Part),
            Some("TConstruct Tinkers Chest") => Some(ChestKind::Tinkers),
            _ => None,
        };
    }
    let key = game.container?;
    match &*game.res.facts().get(game.world.block(key.0, key.1, key.2)).name {
        "tconstruct:part_chest" => Some(ChestKind::Part),
        "tconstruct:tinkers_chest" => Some(ChestKind::Tinkers),
        _ => None,
    }
}

pub(super) fn page(game: &Game<'_>) -> Option<(usize, usize)> {
    let kind = kind(game)?;
    if game.remote.is_some() {
        let key=super::remote::tconstruct_source_machine_remote::position(game)?;
        return game.world.chunks.__horizon_mixins_tconstruct_chest_ui_pages().get(&key).copied();
    }
    let key = game.container?;
    let BlockEntity::Chest(slots) = game.world.block_entities.get(&key)? else { return None; };
    if slots.len() != kind.slots() { return None; }
    let count = if kind == ChestKind::Part { rules::part_visual_size(slots.iter().map(Option::is_some), slots.len()) } else { slots.len() };
    let scroll = game.world.chunks.__horizon_mixins_tconstruct_builder_side_scroll().get(&key).map_or(0, |state| state.0).min(rules::max_scroll(count));
    Some((count, scroll))
}

pub(super) fn position(game: &Game<'_>, slot: usize) -> Option<(f32,f32)> {
    let (count, scroll) = page(game)?;
    match slot {
        0..=8 => Some((8.0 + slot as f32 * 18.0, 160.0)),
        9..=35 => Some((8.0 + ((slot - 9) % 9) as f32 * 18.0, 102.0 + ((slot - 9) / 9) as f32 * 18.0)),
        300..=555 => {
            let index=if game.remote.is_some() { scroll*rules::CHEST_COLUMNS+slot-300 } else { slot-300 };
            Some(rules::visible_slot(count, scroll, index).unwrap_or((-10000.0,-10000.0)))
        },
        _ => None,
    }
}

pub(super) fn set_scroll(game: &mut Game<'_>, next: usize) {
    let Some((count,_))=page(game) else { return; };
    let next=next.min(rules::max_scroll(count));
    if game.remote.is_some() {
        if let Some((window,_))=game.remote.as_ref().and_then(|r|r.menu.as_ref()) {
            game.send(horizon_protocol::packets::play::Serverbound::ContainerButtonClick(horizon_protocol::packets::play::ContainerButtonClick { container:*window,button:41000+next as i32 }));
        }
    } else if let Some(pos)=game.container {
        game.world.chunks.__horizon_mixins_tconstruct_builder_side_scroll_mut().entry(pos).or_default().0=next;
    }
}

pub(super) fn draw(game: &Game<'_>, batch: &mut horizon_client::hud::HudBatch, ox: f32, oy: f32) {
    let Some((count, scroll)) = page(game) else { return; };
    let atlas = &game.gui.atlas;
    batch.sprite_region(atlas, "gui/tconstruct_blank_extra_row", [ox,oy,176.0,184.0], 256.0,256.0);
    if let Some(sprite) = atlas.sprites.get("gui/tconstruct_generic").copied() {
        let region = |batch: &mut horizon_client::hud::HudBatch, sx: f32, sy: f32, sw: f32, sh: f32, x: f32, y: f32, w: f32, h: f32| {
            let mut r = sprite;
            let (du,dv) = ((r.u1-r.u0)/r.w as f32,(r.v1-r.v0)/r.h as f32);
            r.u0 += sx*du; r.v0 += sy*dv; r.u1 = r.u0 + sw*du; r.v1 = r.v0 + sh*dv;
            batch.rect_uv(ox+x,oy+y,w,h,r,u32::MAX);
        };
        let first = scroll * rules::CHEST_COLUMNS;
        for i in 0..rules::CHEST_COLUMNS * rules::CHEST_ROWS {
            let source_x = if first + i < count { 7.0 } else { 25.0 };
            region(batch, source_x,7.0,18.0,18.0,7.0+(i%8) as f32*18.0,17.0+(i/8) as f32*18.0,18.0,18.0);
        }
        region(batch,43.0,8.0,12.0,30.0,157.0,17.0,12.0,72.0);
        let max=rules::max_scroll(count);
        let progress=if max==0 { 0.0 } else { scroll as f32/max as f32 };
        region(batch,7.0,25.0,10.0,15.0,158.0,17.0+progress*57.0,10.0,15.0);
    }
    super::tconstruct_source_workstation_tabs_ui::draw(game,batch,ox,oy,false);
    let (key, fallback) = match kind(game) { Some(ChestKind::Part) => ("gui.tconstruct.part_chest","Part Chest"), _ => ("gui.tconstruct.tinkers_chest","Tinker's Chest") };
    let color=horizon_client::hud::rgba(64,64,64,255);
    batch.text(atlas,&game.translate(key,fallback),ox+8.0,oy+6.0,color,false);
    batch.text(atlas,&game.translate("container.inventory","Inventory"),ox+8.0,oy+90.0,color,false);
}
