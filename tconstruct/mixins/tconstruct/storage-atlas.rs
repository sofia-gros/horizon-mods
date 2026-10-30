fn tconstruct_storage_atlas(assets: &Assets, root: &Path) -> std::ops::ControlFlow<Result<RenderAssets>> {
    let storage = root.join("assets/tconstruct/textures/block/storage");
    if !storage.is_dir() { return std::ops::ControlFlow::Continue(()); }
    let result = (|| -> crate::Result<RenderAssets> {
        let mut sprites = crate::sprites::Sprites::default();
        for name in MATERIALS { sprites.insert(root, &format!("minecraft:block/{name}"))?; }
        let parsed = crate::model_library::ParsedModels::load(root);
        let models = crate::models::prepare_with_models(root, assets.registry, &mut sprites, &parsed)?;
        let model_library = parsed.into_item_library();
        for name in ["water_still", "water_flow", "water_overlay", "lava_still", "lava_flow"] {
            sprites.insert(root, &format!("minecraft:block/{name}"))?;
        }
        for stage in 0..10 { sprites.insert(root, &format!("minecraft:block/destroy_stage_{stage}"))?; }
        let mut files: Vec<_> = std::fs::read_dir(storage)?.collect::<std::io::Result<_>>()?;
        files.sort_by_key(|entry| entry.file_name());
        for entry in files {
            let name = entry.file_name();
            let Some(name) = name.to_str().and_then(|name| name.strip_suffix(".png")) else { continue; };
            if name.starts_with("fallback_") { sprites.insert(root, &format!("tconstruct:block/storage/{name}"))?; }
        }
        // Connected models reference their base sprite during baking. The
        // adjacent variants are selected later while meshing a block face.
        for base in ["clear_glass", "clear_stained_glass", "smeltery/seared_glass", "smeltery/soul_glass", "foundry/glass", "foundry/soul_glass"] {
            let dir = root.join("assets/tconstruct/textures/block").join(base);
            if !dir.is_dir() { continue; }
            for suffix in ["u", "d", "l", "r", "ud", "ul", "ur", "dl", "dr", "lr", "udl", "udr", "ulr", "dlr", "udlr"] {
                sprites.insert(root, &format!("tconstruct:block/{base}/{suffix}"))?;
            }
        }
        sprites.finish();
        sprites.pixels.shrink_to_fit();
        Ok(RenderAssets {
            sprite_size: sprites.size, sprite_minimum_size: sprites.minimum_size,
            texture_layers: u32::try_from(sprites.count())?, pixels: sprites.pixels,
            model_catalog: models.catalog, model_library, model_placements: models.placements,
            model_gallery: models.states, animations: sprites.animations,
            sprite_layers: sprites.layers, translucent_layers: sprites.translucent, sprite_mips: sprites.mips,
        })
    })();
    std::ops::ControlFlow::Break(result)
}
