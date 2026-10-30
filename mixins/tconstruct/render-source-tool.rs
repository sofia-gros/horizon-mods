fn render_source_tool_gui(
    game: &crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    stack: &horizon_client::items::ItemStack,
    x: f32,
    y: f32,
) -> std::ops::ControlFlow<()> {
    use horizon_assets::item_model::DisplayContext;
    use horizon_client::item_render::{self, LayerGeometry};
    use horizon_world::nbt::Tag;
    use std::ops::ControlFlow::{Break, Continue};
    let Some(name) = stack.item.strip_prefix("tconstruct:") else { return Continue(()); };
    let models = game.world.chunks.__horizon_mixins_tconstruct_tool_models().get_or_init(|| {
        let root = game.options.resources.join("assets/tconstruct/tinkering/source_tool_models");
        let mut result = std::collections::HashMap::new();
        for entry in std::fs::read_dir(root).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().and_then(|x| x.to_str()) != Some("json") { continue; }
            let name = path.file_stem().and_then(|x| x.to_str()).ok_or("invalid tool model name")?.to_owned();
            let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            let model = crate::game::tconstruct_source_tool_model::ToolModel::parse(&bytes).ok_or("invalid source tool model")?;
            result.insert(name, model.layers.into_iter().map(|v| (v.texture, v.material_index)).collect());
        }
        Ok(result)
    });
    let Ok(models) = models else { return Continue(()); };
    let Some(layers) = models.get(name) else { return Continue(()); };
    let Some(Tag::List(materials)) = stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|tag| tag.get("tic_materials")) else { return Continue(()); };
    let Ok((records, _)) = game.world.chunks.__horizon_mixins_tconstruct_render_assets()
        .get_or_init(|| crate::game::tconstruct_source_render_info::load_render_assets(&game.options.resources)) else { return Continue(()); };
    let owner = game.player_owner(Some(stack));
    let level = game.item_level();
    let mut states = Vec::with_capacity(layers.len());
    for (base, index) in layers {
        let Some(Tag::String(raw)) = materials.get(*index) else { return Continue(()); };
        let Some(material_id) = crate::game::tconstruct_source_render_info::material_id(&raw.to_string_lossy()) else { return Continue(()); };
        let info = match crate::game::tconstruct_source_render_info::render_info(records, &material_id) {
            Ok(Some(info)) => info,
            Ok(None) => crate::game::tconstruct_source_render_info::RenderInfo { texture: None, fallbacks: Vec::new(), color: u32::MAX, luminosity: 0 },
            Err(_) => return Continue(()),
        };
        let Ok((sprite, color, luminosity)) = crate::game::tconstruct_source_render_info::select_sprite_by(base, &info, |id| game.gui.atlas.sprites.contains_key(id)) else { return Continue(()); };
        let Some(sprite_path) = sprite.strip_prefix("tconstruct:") else { return Continue(()); };
        let alias = format!("tconstruct_tool/{sprite_path}");
        if !game.options.resources.join("assets/tconstruct/items").join(format!("{alias}.json")).is_file() { return Continue(()); }
        let mut rendered = stack.clone();
        rendered.data_mut().set_other("minecraft:item_model", Some(Tag::String(format!("tconstruct:{alias}").into())));
        let mut state = game.pack.item_renderer.resolve(&item_render::stack_view(&rendered), DisplayContext::Gui, Some(&owner), Some(&level), 0, &game.item_env());
        if state.is_empty() { return Continue(()); }
        let Ok(mut frame_cache) = game.world.chunks.__horizon_mixins_tconstruct_render_sprite_frames().lock() else { return Continue(()); };
        if !frame_cache.contains_key(&sprite) {
            let Ok(frames) = crate::game::tconstruct_source_sprite_frames::load(&game.options.resources, &sprite) else { return Continue(()); };
            frame_cache.insert(sprite.clone(), frames);
        }
        let Some((width, height, frames)) = frame_cache.get(&sprite) else { return Continue(()); };
        let Ok(quads) = crate::game::tconstruct_source_mantle_layer::source_part_quads(*width, *height, frames, [0.0, 0.0]) else { return Continue(()); };
        drop(frame_cache);
        let material = horizon_assets::model::Material { sprite: sprite.clone(), force_translucent: false };
        let mapped: Vec<_> = quads.into_iter().map(|quad| {
            use crate::game::tconstruct_source_mantle_layer::Side;
            use horizon_assets::model::Direction;
            let facing = match quad.side { Side::Up => Direction::Up, Side::Down => Direction::Down, Side::West => Direction::West, Side::East => Direction::East, Side::North => Direction::North, Side::South => Direction::South };
            horizon_assets::bake::ModelQuad { positions: quad.positions, uv: quad.uv, facing, cull: None, material: material.clone(), tint_index: 0, shade_direction_override: None, light_emission: luminosity }
        }).collect();
        let extents: Vec<_> = mapped.iter().flat_map(|q| q.positions.iter().map(|p| glam::Vec3::from_array(*p))).collect();
        for layer in &mut state.layers {
            let LayerGeometry::Quads(model) = &mut layer.geometry else { return Continue(()); };
            std::sync::Arc::make_mut(model).quads = mapped.clone();
            layer.extents = std::sync::Arc::new(extents.clone());
            layer.tints = vec![color as i32];
        }
        state.animated = true;
        states.push(state);
    }
    for state in states {
        item_render::gui::draw(&state, &game.pack.item_renderer, &game.gui.atlas.sprites, &game.item_host(), batch, x, y,
            game.started.elapsed().as_secs_f32(), game.settings.file.double("glintSpeed") as f32, game.settings.file.double("glintStrength") as f32);
    }
    Break(())
}
