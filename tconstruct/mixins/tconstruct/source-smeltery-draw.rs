use super::Game;
use horizon_client::hud::{HudBatch, rgba};

pub(super) fn draw(game: &Game<'_>, batch: &mut HudBatch, ox: f32, oy: f32) {
    let atlas = &game.gui.atlas;
    batch.sprite_region(atlas, "gui/tconstruct_heating_structure", [ox, oy, 176.0, 220.0], 256.0, 256.0);
    let color = rgba(64, 64, 64, 255);
    let foundry = super::tconstruct_source_smeltery_ui::is_foundry(game);
    let title = game.pack.names.display_get(if foundry { "container.tconstruct.foundry" } else { "container.tconstruct.smeltery" })
        .unwrap_or_else(|| if foundry { "Foundry".to_owned() } else { "Smeltery".to_owned() });
    let inventory = game.pack.names.display_get("container.inventory").unwrap_or_else(|| "Inventory".to_owned());
    batch.text(atlas, &title, ox + 8.0, oy + 6.0, color, false);
    batch.text(atlas, &inventory, ox + 8.0, oy + 128.0, color, false);
    let mode = super::tconstruct_source_smeltery_ui::position(game)
        .and_then(|pos| game.world.chunks.__horizon_mixins_tconstruct_smeltery_local_containers().get(&pos).map(|(_, mode)| *mode)).unwrap_or(0).min(2);
    if let Some(mut sprite) = atlas.sprites.get("gui/tconstruct_heating_structure").copied() {
        let (du, dv) = ((sprite.u1 - sprite.u0) / sprite.w as f32, (sprite.v1 - sprite.v0) / sprite.h as f32);
        let original = sprite;
        if game.container_kind().is_some_and(|entity| entity.slots().first().is_none_or(Option::is_none)) {
            let mut icon = original;
            icon.u0 += 224.0 * du; icon.v0 += 186.0 * dv;
            icon.u1 = icon.u0 + 16.0 * du; icon.v1 = icon.v0 + 16.0 * dv;
            batch.rect_uv(ox + 125.0, oy + 46.0, 16.0, 16.0, icon, u32::MAX);
        }
        if (ox + 124.0..ox + 142.0).contains(&game.cursor.0) && (oy + 69.0..oy + 87.0).contains(&game.cursor.1) {
            let mut hover = original;
            hover.u0 += 176.0 * du; hover.v0 += 202.0 * dv;
            hover.u1 = hover.u0 + 18.0 * du; hover.v1 = hover.v0 + 18.0 * dv;
            batch.rect_uv(ox + 124.0, oy + 69.0, 18.0, 18.0, hover, u32::MAX);
        }
        sprite.u0 += (176.0 + f32::from(mode) * 16.0) * du; sprite.v0 += 186.0 * dv;
        sprite.u1 = sprite.u0 + 16.0 * du; sprite.v1 = sprite.v0 + 16.0 * dv;
        batch.rect_uv(ox + 125.0, oy + 70.0, 16.0, 16.0, sprite, u32::MAX);
    }
    let (count, offset) = super::tconstruct_source_smeltery_ui::page(game);
    let cols = super::tconstruct_source_smeltery_layout::columns(count);
    let rows = super::tconstruct_source_smeltery_layout::visible_rows(count);
    let max = super::tconstruct_source_smeltery_layout::max_offset(count);
    let width = super::tconstruct_source_smeltery_layout::panel_width(count);
    if cols > 0 {
        let left = -width;
        let inner_width = width - 14.0;
        let inner_height = rows as f32 * 18.0;
        if let Some(sprite) = atlas.sprites.get("gui/tconstruct_generic").copied() {
            let region = |batch: &mut HudBatch, sx: f32, sy: f32, sw: f32, sh: f32, x: f32, y: f32, w: f32, h: f32| {
                let mut r = sprite;
                let (du, dv) = ((r.u1-r.u0)/r.w as f32, (r.v1-r.v0)/r.h as f32);
                r.u0 += sx*du; r.v0 += sy*dv; r.u1 = r.u0+sw*du; r.v1 = r.v0+sh*dv;
                batch.rect_uv(ox+left+x, oy+4.0+y, w, h, r, u32::MAX);
            };
            for (sx, sy, x, y) in [(0.0,0.0,0.0,0.0),(57.0,0.0,inner_width+7.0,0.0),
                (0.0,57.0,0.0,inner_height+7.0),(57.0,57.0,inner_width+7.0,inner_height+7.0)] {
                region(batch,sx,sy,7.0,7.0,x,y,7.0,7.0);
            }
            region(batch,7.0,0.0,50.0,7.0,7.0,0.0,inner_width,7.0);
            region(batch,7.0,57.0,50.0,7.0,7.0,inner_height+7.0,inner_width,7.0);
            region(batch,0.0,7.0,7.0,50.0,0.0,7.0,7.0,inner_height);
            region(batch,57.0,7.0,7.0,50.0,inner_width+7.0,7.0,7.0,inner_height);
            if max > 0 {
                let x = 7.0 + cols as f32 * 22.0;
                region(batch,43.0,8.0,12.0,30.0,x,7.0,10.0,inner_height);
                let progress = offset.min(max) as f32 / max as f32;
                region(batch,7.0,25.0,10.0,15.0,x,7.0+progress*(inner_height-15.0).max(0.0),10.0,15.0);
            }
        }
        if let Some(sprite) = atlas.sprites.get("gui/tconstruct_heating_structure").copied() {
            let du = (sprite.u1-sprite.u0)/sprite.w as f32; let dv = (sprite.v1-sprite.v0)/sprite.h as f32;
            let first = offset.min(max) * cols;
            for i in 0..rows*cols {
                let mut r = sprite; r.u0 += if first+i < count { 0.0 } else { 22.0*du }; r.v0 += 238.0*dv;
                r.u1 = r.u0+22.0*du; r.v1 = r.v0+18.0*dv;
                batch.rect_uv(ox+left+7.0+(i%cols) as f32*22.0, oy+11.0+(i/cols) as f32*18.0, 22.0,18.0,r,u32::MAX);
            }
        }
    }
    let tank = super::tconstruct_source_smeltery_ui::position(game)
        .and_then(|pos| super::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery"))
        .and_then(|tag| tag.get("tank").and_then(super::tconstruct_source_smeltery_tank::SmelteryTank::from_nbt));
    if let Some(tank) = tank.filter(|tank| tank.capacity > 0) {
        let mut y = 0.0f32;
        let amounts = tank.fluids.iter().map(|fluid| fluid.amount).collect::<Vec<_>>();
        let heights = super::tconstruct_source_smeltery_layout::fluid_heights(&amounts, tank.capacity);
        for (fluid, height) in tank.fluids.iter().zip(heights) {
            let height = height as f32;
            if height <= 0.0 { continue; }
            let name = crate::game::tconstruct_source_melter_render::sprite(&fluid.id);
            if let Some(sprite) = atlas.sprites.get(&name).or_else(|| atlas.sprites.get("block/tconstruct_fluid_fallback")).copied() {
                let (du,dv)=((sprite.u1-sprite.u0)/sprite.w as f32,(sprite.v1-sprite.v0)/sprite.h as f32);
                let mut yy=0.0;
                while yy<height { let h=(height-yy).min(16.0); let mut x=0.0;
                    while x<106.0 { let w=(106.0f32-x).min(16.0); let mut part=sprite;
                        part.u1=part.u0+w*du; part.v1=part.v0+h*dv;
                        batch.rect_uv(ox+8.0+x,oy+122.0-y-yy-h,w,h,part,super::tconstruct_source_melter_render::color(&fluid.id)); x+=w;
                    } yy+=h;
                }
            }
            y += height;
        }
    }
    // HeatingStructureScreen overlays its original 80×106 gauge at the
    // tank's upper left after drawing its fluid layers.
    if let Some(mut gauge) = atlas.sprites.get("gui/tconstruct_heating_structure").copied() {
        let (du, dv) = ((gauge.u1 - gauge.u0) / gauge.w as f32, (gauge.v1 - gauge.v0) / gauge.h as f32);
        gauge.u0 += 176.0 * du;
        gauge.u1 = gauge.u0 + 80.0 * du; gauge.v1 = gauge.v0 + 106.0 * dv;
        batch.rect_uv(ox + 8.0, oy + 16.0, 80.0, 106.0, gauge, u32::MAX);
    }
    if (ox + 8.0..ox + 114.0).contains(&game.cursor.0) && (oy + 16.0..oy + 122.0).contains(&game.cursor.1) {
        let tank = super::tconstruct_source_smeltery_ui::position(game)
            .and_then(|pos| super::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery"))
            .and_then(|tag| tag.get("tank").and_then(super::tconstruct_source_smeltery_tank::SmelteryTank::from_nbt));
        if let Some(tank) = tank {
            let amounts = tank.fluids.iter().map(|fluid| fluid.amount).collect::<Vec<_>>();
            let heights = super::tconstruct_source_smeltery_layout::fluid_heights(&amounts, tank.capacity);
            let index = super::tconstruct_source_smeltery_layout::fluid_at(&amounts, tank.capacity, game.cursor.1 - oy);
            let (top, height) = if let Some(index) = index {
                let below: u32 = heights.iter().take(index).sum();
                let height = heights[index] as f32;
                (oy + 122.0 - below as f32 - height, height)
            } else {
                (oy + 16.0, 106.0 - heights.iter().sum::<u32>() as f32)
            };
            if height > 0.0 { batch.fill(atlas, ox + 8.0, top, 106.0, height, rgba(255, 255, 255, 128)); }
        }
    }
    let data = super::tconstruct_source_smeltery_ui::position(game)
        .and_then(|pos| super::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery"));
    let view = data.as_ref().and_then(|tag| tag.get("fuel_view"))
        .and_then(super::tconstruct_source_smeltery_fuel::FuelView::from_nbt);
    if let Some(view) = view {
        if let Some(fluid) = view.fluid.as_deref() {
            if view.capacity > 0 {
                let height = (90u64 * u64::from(view.amount) / u64::from(view.capacity)).min(90) as f32;
                let sprite = super::tconstruct_source_melter_render::sprite(fluid);
                if let Some(sprite) = atlas.sprites.get(&sprite).or_else(|| atlas.sprites.get("block/tconstruct_fluid_fallback")).copied() {
                    let (du, dv) = ((sprite.u1 - sprite.u0) / sprite.w as f32, (sprite.v1 - sprite.v0) / sprite.h as f32);
                    let mut y = 0.0;
                    while y < height {
                        let h = (height - y).min(16.0);
                        let mut part = sprite; part.u1 = part.u0 + 16.0 * du; part.v1 = part.v0 + h * dv;
                        batch.rect_uv(ox + 152.0, oy + 122.0 - y - h, 16.0, h, part, super::tconstruct_source_melter_render::color(fluid));
                        y += h;
                    }
                }
            }
        }
        if (ox + 151.0..ox + 169.0).contains(&game.cursor.0) && (oy + 31.0..oy + 123.0).contains(&game.cursor.1) {
            batch.fill(atlas, ox + 152.0, oy + 32.0, 16.0, 90.0, rgba(255, 255, 255, 128));
        }
    }
    let number = |key: &str| data.as_ref().and_then(|tag| tag.get(key)).and_then(|value| match value {
        horizon_world::nbt::Tag::Long(value) => u32::try_from(*value).ok(), _ => None,
    }).unwrap_or(0);
    super::tconstruct_source_melter_render::draw_fire(batch, atlas, "gui/tconstruct_heating_structure", ox + 153.0, oy + 15.0, number("fuel"), number("fuel_quality"));
    for (code, x, y) in super::tconstruct_source_smeltery_ui::visible_heat(game) {
        if let Some((sprite, height, _)) = super::tconstruct_source_melter_render::decode_heat(code) {
            super::tconstruct_source_melter_render::draw_heat_bar(batch, atlas, "gui/tconstruct_heating_structure", ox + x, oy + y, sprite, height);
        }
    }
}
