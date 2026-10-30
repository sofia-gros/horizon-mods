use super::*;

pub(super) const TEXT_SCALE: f32 = 7.0 / 9.0;
pub(super) const LINE_HEIGHT: f32 = 7.5;
pub(super) const VISIBLE_LINES: usize = 23;

pub(super) fn visible_index(entries: &[(String, u32, Option<String>)], scroll: usize, row: usize) -> Option<usize> {
    let headers = entries.iter().enumerate().take_while(|(index, (_, color, _))|
        *index == 0 || *color == horizon_client::hud::rgba(255, 170, 0, 255) || *color == horizon_client::hud::rgba(127, 255, 255, 255)).count();
    let index = if row < headers { row } else { row + scroll.min(entries.len().saturating_sub(VISIBLE_LINES)) };
    (index < entries.len()).then_some(index)
}

/// Text uses the authoritative local recipe or the server's menu data.
pub(super) fn lines(game: &Game<'_>, atlas: &horizon_client::hud::HudAtlas) -> Vec<(String, u32, Option<String>)> {
    let Some(pos) = game.container else { return Vec::new(); };
    let info = if game.remote.is_some() {
        game.world.chunks.__horizon_mixins_tconstruct_builder_info().get(&pos).cloned()
    } else {
        Some(super::tconstruct_source_builder_world::display_info(&game.world, pos))
    };
    let selected = game.world.chunks.__horizon_mixins_tconstruct_part_selections().get(&pos);
    let material = info.as_ref().and_then(|info| info.get(1)).filter(|id| id.as_str() != "-");
    let title = info.as_ref().and_then(|info| info.get(5)).filter(|title| title.as_str() != "-");
    let caption_key = if let Some(title) = title {
        title.split_once(':').map(|(namespace, path)| format!("recipe.{namespace}.{}", path.replace('/', ".")))
    } else if let Some(material) = material {
        material.split_once(':').map(|(namespace, path)| format!("material.{namespace}.{}", path.split('#').next().unwrap_or(path).replace('/', ".")))
    } else { Some("gui.tconstruct.part_builder".to_owned()) };
    let caption = caption_key.map(|key| game.pack.names.display_get(&key).unwrap_or(key)).unwrap_or_default();
    let mut lines = vec![(caption, u32::MAX, None)];
    if let Some(info) = info.as_ref() {
        if selected.is_some() && info.first().is_some_and(|cost| cost != "0") {
            let label = game.pack.names.display_get("gui.tconstruct.part_builder.cost").unwrap_or_else(|| "Pattern cost: %s".into());
            lines.push((label.replace("%s", &info[0]), horizon_client::hud::rgba(255, 170, 0, 255), None));
        }
        if material.is_some() {
            let label = game.pack.names.display_get("gui.tconstruct.part_builder.material_value").unwrap_or_else(|| "Material value: %s".into());
            lines.push((label.replace("%s", &info[2]), horizon_client::hud::rgba(127, 255, 255, 255), None));
            let stats = info.get(3).and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok());
            let traits = info.get(4).and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok());
            if let Some(stats) = stats.as_ref().and_then(serde_json::Value::as_object) {
                for (stat, fields) in stats {
                    if stat == "tconstruct:repair_kit" { continue; }
                    let Some(fields) = fields.as_object() else { continue; };
                    if let Some((namespace, name)) = stat.split_once(':') {
                        let key = format!("stat.{namespace}.{name}");
                        lines.push((game.pack.names.display_get(&key).unwrap_or(key), u32::MAX, None));
                    }
                    if fields.is_empty() {
                        if let Some(label) = game.pack.names.display_get("tool_stat.tconstruct.extra.no_stats") { lines.push((label, u32::MAX, None)); }
                    }
                    let mut ordered: Vec<_> = fields.iter().collect();
                    let order = super::tconstruct_source_builder_display::field_order(stat);
                    ordered.sort_by_key(|(field, _)| order.iter().position(|known| *known == field.as_str()).unwrap_or(usize::MAX));
                    for (field, value) in ordered {
                        let raw_field = field.as_str();
                        let field = match raw_field { "melee_attack" | "melee_damage" => "attack_damage", "melee_speed" => "attack_speed", "mining_tier" => "harvest_tier", other => other };
                        let key = format!("tool_stat.tconstruct.{field}");
                        if let Some(label) = game.pack.names.display_get(&key) {
                            let Some(mut value) = super::tconstruct_source_builder_display::value(stat, raw_field, value) else { continue; };
                            if raw_field == "mining_tier" {
                                if let Some((namespace, tier)) = value.split_once(':') {
                                    value = game.pack.names.display_get(&format!("stat.tconstruct.harvest_tier.{namespace}.{tier}")).unwrap_or(value.clone());
                                }
                            }
                            let tip_key = if stat == "tconstruct:handle" { format!("tool_stat.tconstruct.handle.{field}.description") } else { format!("{key}.description") };
                            let tip = game.pack.names.display_get(&tip_key);
                            lines.push((format!("{label}{value}"), u32::MAX, tip));
                        }
                    }
                    if let Some(traits) = traits.as_ref().and_then(|traits| traits.get(stat)).and_then(serde_json::Value::as_object) {
                        for (id, level) in traits {
                            if let Some((namespace, name)) = id.split_once(':') {
                                let key = format!("modifier.{namespace}.{name}");
                                let label = game.pack.names.display_get(&key).unwrap_or_else(|| key.clone());
                                let tip = game.pack.names.display_get(&format!("{key}.description")).map(|tip| tip.replace("%s", &level.to_string()));
                                lines.push((format!("{label} {level}"), u32::MAX, tip));
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some(title) = title {
        let key = title.split_once(':').map(|(namespace, path)| format!("recipe.{namespace}.{}.info", path.replace('/', ".")));
        let text = key.and_then(|key| game.pack.names.display_get(&key)).unwrap_or_default();
        lines.extend(horizon_client::multiplayer::wrap(&text, 112.0 / TEXT_SCALE, atlas).into_iter().map(|line| (line, u32::MAX, None)));
    } else if material.is_none() {
        let text = game.pack.names.display_get("gui.tconstruct.part_builder.info").unwrap_or_default();
        lines.extend(horizon_client::multiplayer::wrap(&text, 112.0 / TEXT_SCALE, atlas).into_iter().map(|line| (line, u32::MAX, None)));
    }
    lines
}

pub(super) fn panel(game: &Game<'_>, batch: &mut horizon_client::hud::HudBatch, ox: f32, oy: f32, height: f32) {
    let atlas = &game.gui.atlas;
    let Some(sprite) = atlas.sprites.get("gui/tconstruct_panel").copied() else { return; };
    let region = |batch: &mut horizon_client::hud::HudBatch, sx: f32, sy: f32, sw: f32, sh: f32, x: f32, y: f32, w: f32, h: f32| {
        let mut r = sprite;
        let (du, dv) = ((r.u1 - r.u0) / r.w as f32, (r.v1 - r.v0) / r.h as f32);
        r.u0 += sx * du; r.v0 += sy * dv; r.u1 = r.u0 + sw * du; r.v1 = r.v0 + sh * dv;
        batch.rect_uv(ox + x, oy + y, w, h, r, u32::MAX);
    };
    let inner = height - 8.0;
    region(batch, 4.0, 4.0, 118.0, 75.0, 4.0, 4.0, 118.0, inner);
    for (sx, sy, x, y) in [(0.0,0.0,0.0,0.0),(122.0,0.0,122.0,0.0),(0.0,79.0,0.0,height - 4.0),(122.0,79.0,122.0,height - 4.0)] {
        region(batch, sx, sy, 4.0, 4.0, x, y, 4.0, 4.0);
    }
    region(batch, 4.0, 0.0, 118.0, 4.0, 4.0, 0.0, 118.0, 4.0);
    region(batch, 4.0, 79.0, 118.0, 4.0, 4.0, height - 4.0, 118.0, 4.0);
    region(batch, 0.0, 4.0, 4.0, 75.0, 0.0, 4.0, 4.0, inner);
    region(batch, 122.0, 4.0, 4.0, 75.0, 122.0, 4.0, 4.0, inner);
}

pub(super) fn draw(game: &Game<'_>, batch: &mut horizon_client::hud::HudBatch, ox: f32, oy: f32) {
    let atlas = &game.gui.atlas;
    panel(game, batch, ox + 180.0, oy, 184.0);
    let entries = lines(game, atlas);
    let scroll = game.container.and_then(|pos| game.world.chunks.__horizon_mixins_tconstruct_builder_info_scroll().get(&pos)).copied().unwrap_or(0).min(entries.len().saturating_sub(VISIBLE_LINES));
    if entries.iter().any(|(_, _, tip)| tip.is_some()) { batch.text(atlas, "?", ox + 297.0, oy + 5.0, horizon_client::hud::rgba(95, 95, 95, 255), false); }
    for index in 0..VISIBLE_LINES {
        let Some(entry_index) = visible_index(&entries, scroll, index) else { break; };
        let (text, color, _) = &entries[entry_index];
        let centered = entry_index == 0 || *color == horizon_client::hud::rgba(255, 170, 0, 255) || *color == horizon_client::hud::rgba(127, 255, 255, 255);
        let x = if centered { ox + 243.0 - horizon_client::hud::HudBatch::text_width(atlas, text) * TEXT_SCALE / 2.0 } else { ox + 185.0 };
        let y = oy + 5.0 + index as f32 * LINE_HEIGHT;
        let start = batch.vertices.len();
        batch.text(atlas, text, x, y, *color, true);
        batch.scale_since(start, x, y, TEXT_SCALE, TEXT_SCALE);
    }
}
