// Station equipment preview using Horizon's armor-stand models and UI renderer.
use super::{Game, player_preview_quads};
use horizon_client::{hud::HudBatch, mobs::{anim::Poses, render}, mesher::Vertex};

pub(in crate::game) fn draw(game: &Game<'_>, batch: &mut HudBatch, ox: f32, oy: f32) {
    let assets = &game.armor_stand_assets;
    let Some(model) = assets.main.as_ref() else { return; };
    let sprites = &game.gui.atlas.sprites;
    let Some(wood) = sprites.get("entity/armorstand/armorstand") else { return; };
    let uv = |r: &horizon_client::hud::Rect| [r.u0, r.v0, r.u1, r.v1];
    let base = glam::Mat4::from_rotation_y(std::f32::consts::PI)
        * glam::Mat4::from_scale(glam::Vec3::new(-1.0, -1.0, 1.0))
        * glam::Mat4::from_translation(glam::Vec3::new(0.0, -1.501, 0.0));
    let mut verts: Vec<Vertex> = Vec::new();
    let light = 240 | 240 << 8;
    render::draw(model, base, &Poses::new(model), uv(wood), u32::MAX, light, 0, &mut verts);
    for (index, layer) in ["helmet", "chestplate", "leggings", "boots"].into_iter().enumerate() {
        let Some(stack) = game.armor[index].as_ref() else { continue; };
        let Some(asset) = game.res.items().get(&stack.item).and_then(|item| item.equip_asset.as_ref()) else { continue; };
        let kind = if layer == "leggings" { "humanoid_leggings" } else { "humanoid" };
        let Some((texture, color)) = assets.equipment.get(asset).and_then(|entry| entry.get(kind)) else { continue; };
        let (Some(sprite), Some(part)) = (sprites.get(texture), assets.layers.get(&(layer, false))) else { continue; };
        let color = color.map_or(u32::MAX, |c| 0xFF00_0000 | (c & 0xFF) << 16 | (c & 0xFF00) | (c >> 16) & 0xFF);
        render::draw(part, base, &Poses::new(part), uv(sprite), color, light, 0, &mut verts);
    }
    let (cx, cy, size) = (ox - 55.0, oy + 195.0, 35.0);
    let matrix = glam::Mat4::from_translation(glam::Vec3::new(cx, cy, 0.0))
        * glam::Mat4::from_scale(glam::Vec3::new(size, size, -size))
        * glam::Mat4::from_translation(glam::Vec3::new(0.0, 1.0, 0.0))
        * glam::Mat4::from_rotation_z(std::f32::consts::PI);
    let start = batch.vertices.len();
    for (_, vertices, tex, color) in player_preview_quads(verts, matrix, None, sprites) {
        batch.quad_3d(vertices, tex, color);
    }
    batch.depth_since(start, [cx - 35.0, cy - 70.0, cx + 35.0, cy]);
}
