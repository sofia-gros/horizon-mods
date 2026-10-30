use super::*;

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
                    for (field, value) in fields {
                        let field = match field.as_str() { "melee_attack" => "attack_damage", "mining_tier" => "harvest_tier", other => other };
                        let key = format!("tool_stat.tconstruct.{field}");
                        if let Some(label) = game.pack.names.display_get(&key) {
                            let value = value.as_str().map(str::to_owned).unwrap_or_else(|| value.to_string());
                            let tip = game.pack.names.display_get(&format!("{key}.description"));
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
        lines.extend(horizon_client::multiplayer::wrap(&text, 112.0, atlas).into_iter().map(|line| (line, u32::MAX, None)));
    } else if material.is_none() {
        let text = game.pack.names.display_get("gui.tconstruct.part_builder.info").unwrap_or_default();
        lines.extend(horizon_client::multiplayer::wrap(&text, 112.0, atlas).into_iter().map(|line| (line, u32::MAX, None)));
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
    let scroll = game.container.and_then(|pos| game.world.chunks.__horizon_mixins_tconstruct_builder_info_scroll().get(&pos)).copied().unwrap_or(0).min(entries.len().saturating_sub(17));
    if entries.iter().any(|(_, _, tip)| tip.is_some()) { batch.text(atlas, "?", ox + 297.0, oy + 5.0, horizon_client::hud::rgba(95, 95, 95, 255), false); }
    for (index, (text, color, _)) in entries.iter().skip(scroll).take(17).enumerate() {
        batch.text(atlas, text, ox + 185.0, oy + 5.0 + index as f32 * 10.0, *color, true);
    }
}
