// Station equipment preview using Horizon's armor-stand models and UI renderer.
use super::Game;
use crate::game::perspective::tconstruct_preview_projection::player_preview_quads;
use horizon_client::{entities::ArmorStand, hud::HudBatch, mobs::{anim::Poses, render}, mesher::Vertex};

// Resolve the same result/operation stack for preview and trim preparation.
pub(in crate::game) fn selected(game: &Game<'_>) -> Option<horizon_client::items::ItemStack> {
    let entity = game.container_kind()?;
    let slots = entity.slots();
    if game.remote.is_some() {
        slots.get(24).cloned().flatten().or_else(|| slots.get(25).cloned().flatten())
    } else {
        crate::game::tconstruct_source_station_world::building_plan(&game.world, slots).map(|(_, output)| output)
            .or_else(|| crate::game::tconstruct_source_station_world::operation_plan(&game.world, slots).map(|plan| plan.output))
    }.or_else(|| crate::game::tconstruct_source_station_world::has_operation_tool(slots).then(|| slots.first().cloned().flatten()).flatten())
}

// Source trim modifiers store registry IDs in persistent tool data.
pub(in crate::game) fn source_trim(game: &Game<'_>, stack: &horizon_client::items::ItemStack) -> Option<horizon_client::trims::TrimTexture> {
    use horizon_world::nbt::Tag;
    if !matches!(stack.item.as_ref(), "tconstruct:plate_helmet" | "tconstruct:plate_chestplate" | "tconstruct:plate_leggings" | "tconstruct:plate_boots") { return None; }
    let persistent = stack.data()?.other("minecraft:custom_data")?.get("tic_persistent")?;
    let Tag::String(pattern) = persistent.get("tconstruct:trim_pattern")? else { return None; };
    let Tag::String(material) = persistent.get("tconstruct:trim_material")? else { return None; };
    let kind = if stack.item.as_ref() == "tconstruct:plate_leggings" { "humanoid_leggings" } else { "humanoid" };
    horizon_client::trims::TrimTexture::resolve(&game.armor_stand_assets.trims, &[], &material.to_string_lossy(), &pattern.to_string_lossy(), kind)
}

pub(in crate::game) fn draw(game: &Game<'_>, batch: &mut HudBatch, ox: f32, oy: f32) {
    let assets = &game.armor_stand_assets;
    let Some(model) = assets.main.as_ref() else { return; };
    let sprites = &game.gui.atlas.sprites;
    let Some(wood) = sprites.get("entity/armorstand/armorstand") else { return; };
    let uv = |r: &horizon_client::hud::Rect| [r.u0, r.v0, r.u1, r.v1];
    let output = selected(game);
    let mut stand = ArmorStand::new(0, horizon_core::geometry::Vec3::ZERO, 210.0);
    stand.show_arms = true; stand.no_base_plate = true;
    let mut layer = None;
    if let Some(stack) = output {
        let item = game.res.items().get(&stack.item);
        layer = item.and_then(|info| match info.equip_slot.as_deref() {
            Some("head") => Some("helmet"), Some("chest") => Some("chestplate"),
            Some("legs") => Some("leggings"), Some("feet") => Some("boots"), _ => None,
        });
        let slot = match layer { Some("helmet") => 5, Some("chestplate") => 4, Some("leggings") => 3, Some("boots") => 2, _ => 1 };
        stand.equipment[slot] = Some(stack);
    }
    let angle = game.container.and_then(|pos| game.world.chunks.__horizon_mixins_tconstruct_station_armor_preview().get(&pos)).map_or(0.0, |state| state.0);
    let base = glam::Mat4::from_rotation_y((-30.0f32).to_radians() + angle)
        * glam::Mat4::from_scale(glam::Vec3::new(-1.0, -1.0, 1.0))
        * glam::Mat4::from_translation(glam::Vec3::new(0.0, -1.501, 0.0));
    let mut poses = Poses::new(model);
    for (names, rotation) in [
        (&["head"][..], stand.pose[0]), (&["body", "right_body_stick", "left_body_stick", "shoulder_stick"][..], stand.pose[1]),
        (&["left_arm"][..], stand.pose[2]), (&["right_arm"][..], stand.pose[3]),
        (&["left_leg"][..], stand.pose[4]), (&["right_leg"][..], stand.pose[5]),
    ] {
        for name in names { if poses.has(name) { let part = poses.get(name); for axis in 0..3 { part[3 + axis] = rotation[axis].to_radians(); } } }
    }
    if poses.has("base_plate") { let part = poses.get("base_plate"); part[6] = 0.0; part[7] = 0.0; part[8] = 0.0; }
    let mut verts: Vec<Vertex> = Vec::new(); let light = 240 | 240 << 8;
    render::draw(model, base, &poses, uv(wood), u32::MAX, light, 0, &mut verts);
    game.armor_stand_items(&stand, model, base, &poses, light, &mut verts);
    if let Some(layer) = layer {
        let slot = match layer { "helmet" => 5, "chestplate" => 4, "leggings" => 3, _ => 2 };
        if let Some(stack) = stand.equipment[slot].as_ref() {
            // Plate armor uses original plating/maille at material indices 0/1.
            // Source-shaped dyed modifier data selects the maille layer; trim remains separate.
            let plate_type = match (stack.item.as_ref(), layer) {
                ("tconstruct:plate_helmet", "helmet") | ("tconstruct:plate_chestplate", "chestplate") | ("tconstruct:plate_boots", "boots") => Some("armor"),
                ("tconstruct:plate_leggings", "leggings") => Some("leggings"),
                _ => None,
            };
            if let Some(texture_type) = plate_type {
                if let (Some(horizon_world::nbt::Tag::List(materials)), Ok((records, _)), Some(part)) = (
                    stack.data().and_then(|data| data.other("minecraft:custom_data")).and_then(|tag| tag.get("tic_materials")),
                    game.world.chunks.__horizon_mixins_tconstruct_render_assets().get_or_init(|| crate::game::tconstruct_source_render_info::load_render_assets(&game.options.resources)),
                    assets.layers.get(&(layer, false)),
                ) {
                    let select = |index: usize, kind: &str| -> Option<_> {
                        let horizon_world::nbt::Tag::String(value) = materials.get(index)? else { return None; };
                        let id = crate::game::tconstruct_source_render_info::material_id(&value.to_string_lossy())?;
                        let info = crate::game::tconstruct_source_render_info::render_info(records, &id).ok()?.unwrap_or(crate::game::tconstruct_source_render_info::RenderInfo { texture: None, fallbacks: Vec::new(), color: u32::MAX, luminosity: 0 });
                        let available = |id: &str| id.strip_prefix("tconstruct:").is_some_and(|path| sprites.contains_key(&format!("gui/tconstruct_plate_{path}")));
                        let custom = stack.data().and_then(|data| data.other("minecraft:custom_data"));
                        let dyed = custom.and_then(|tag| tag.get("tic_modifiers")).and_then(|tag| match tag { horizon_world::nbt::Tag::List(entries) => Some(entries), _ => None }).is_some_and(|entries| entries.iter().any(|entry| {
                            matches!(entry.get("name"), Some(horizon_world::nbt::Tag::String(name)) if name.to_string_lossy() == "tconstruct:dyed")
                                && matches!(entry.get("level"), Some(horizon_world::nbt::Tag::Int(level)) if *level > 0)
                        }));
                        let dye = dyed.then(|| match custom.and_then(|tag| tag.get("tic_persistent")).and_then(|tag| tag.get("tconstruct:dyed")) {
                            Some(horizon_world::nbt::Tag::Int(color)) => *color as u32,
                            _ => u32::MAX,
                        });
                        let dyed_maille = if kind == "maille" { crate::game::tconstruct_source_render_info::select_plate_dyed_maille_by(texture_type, &info, dye, available) } else { None };
                        let (id, color) = if let Some(selected) = dyed_maille { selected } else {
                            let (id, color, _) = crate::game::tconstruct_source_render_info::select_armor_sprite_by(&format!("tconstruct:{kind}_{texture_type}"), &info, available).ok()??;
                            (id, color)
                        };
                        let sprite = sprites.get(&format!("gui/tconstruct_plate_{}", id.strip_prefix("tconstruct:")?))?;
                        let color = (color & 0xFF00_0000) | (color & 0xFF) << 16 | (color & 0xFF00) | (color >> 16) & 0xFF;
                        Some((sprite, color))
                    };
                    if let (Some(plating), Some(maille)) = (select(0, "plating"), select(1, "maille")) {
                        let mut layer_poses = Poses::new(part);
                        super::apply_pose(&mut layer_poses, &stand.pose);
                        for (sprite, color) in [plating, maille] {
                            render::draw(part, base, &layer_poses, uv(sprite), color, light, 0, &mut verts);
                        }
                        if let Some(trim) = source_trim(game, stack).and_then(|trim| sprites.get(&trim.sprite())) {
                            render::draw(part, base, &layer_poses, uv(trim), u32::MAX, light, 0, &mut verts);
                        }
                    }
                }
            }
            if let Some(asset) = game.res.items().get(&stack.item).and_then(|info| info.equip_asset.as_ref()) {
                let kind = if layer == "leggings" { "humanoid_leggings" } else { "humanoid" };
                if let (Some((texture, color)), Some(part)) = (assets.equipment.get(asset).and_then(|entry| entry.get(kind)), assets.layers.get(&(layer, false))) {
                    if let Some(sprite) = sprites.get(texture) {
                        let color = color.map_or(u32::MAX, |c| 0xFF00_0000 | (c & 0xFF) << 16 | (c & 0xFF00) | (c >> 16) & 0xFF);
                        let mut layer_poses = Poses::new(part);
                        super::apply_pose(&mut layer_poses, &stand.pose);
                        render::draw(part, base, &layer_poses, uv(sprite), color, light, 0, &mut verts);
                    }
                }
            }
        }
    }
    let (cx, cy, size) = (ox - 55.0, oy + 195.0, 35.0);
    let matrix = glam::Mat4::from_translation(glam::Vec3::new(cx, cy, 0.0))
        * glam::Mat4::from_scale(glam::Vec3::new(size, size, -size))
        * glam::Mat4::from_rotation_z(std::f32::consts::PI)
        * glam::Mat4::from_rotation_x(25.0f32.to_radians());
    let start = batch.vertices.len();
    for (_, vertices, tex, color) in player_preview_quads(verts, matrix, None, sprites) { batch.quad_3d(vertices, tex, color); }
    batch.depth_since(start, [cx - 35.0, cy - 70.0, cx + 35.0, cy]);
    if let Some(mut icon) = sprites.get("gui/tconstruct_icons").copied() {
        let (du, dv) = ((icon.u1 - icon.u0) / icon.w as f32, (icon.v1 - icon.v0) / icon.h as f32);
        icon.v0 += 184.0 * dv; icon.u1 = icon.u0 + 32.0 * du; icon.v1 = icon.v0 + 32.0 * dv;
        batch.rect_uv(cx - 16.0, cy - 16.0, 32.0, 32.0, icon, u32::MAX);
    }
}
