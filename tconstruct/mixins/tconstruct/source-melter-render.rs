// Vanilla fluids use their existing still sprites; TConstruct fluids use the
// source images staged under a separate prefix.
pub(super) fn sprite(fluid: &str) -> String {
    match fluid {
        "minecraft:lava" => "block/lava_still".to_owned(),
        "minecraft:water" => "block/water_still".to_owned(),
        _ => format!("block/tconstruct_fluid_{}", fluid.strip_prefix("tconstruct:").unwrap_or("")),
    }
}

// Generated from the source FluidTextureProvider ARGB colors.
pub(super) fn color(fluid: &str) -> u32 {
    match fluid {
        "tconstruct:beetroot_soup" => 0xff0d1684,
        "tconstruct:meat_soup" => 0xff353ee0,
        "tconstruct:molten_bendalloy" => 0xffc1d2d6,
        "tconstruct:molten_cadmium" => 0xffc8b88a,
        "tconstruct:molten_chromium" => 0xff80b1c4,
        "tconstruct:molten_clay" => 0xff45609b,
        "tconstruct:molten_debris" => 0xff151e41,
        "tconstruct:molten_ender" => 0xff515e10,
        "tconstruct:mushroom_stew" => 0xff6f8ccd,
        "tconstruct:potion" => 0xfff800f8,
        "tconstruct:rabbit_stew" => 0xff2c4a98,
        "tconstruct:scorched_stone" => 0xff29303e,
        "tconstruct:seared_stone" => 0xff474a4f,
        _ => u32::MAX,
    }
}

/// Source GuiMeltingModule progress strip: sprite x and visible height.
pub(super) fn heat_bar(world: &super::World, stack: &horizon_items::items::ItemStack,
    temperature: u32, progress: i32, foundry: bool) -> (f32, f32, Option<&'static str>) {
    let recipe = if foundry {
        super::tconstruct_source_melter_world::foundry_melting(world, &stack.item).map(|(_, recipe, _, _)| recipe.melting)
    } else {
        super::tconstruct_source_melter_world::melting(world, &stack.item).map(|(_, recipe, _)| recipe)
    };
    let Some(recipe) = recipe else { return (185.0, 16.0, Some("gui.tconstruct.melting.no_recipe")); };
    let required = recipe.time.saturating_mul(10);
    if required == 0 { return (185.0, 16.0, Some("gui.tconstruct.melting.no_recipe")); }
    if temperature < recipe.temperature { return (179.0, 16.0, Some("gui.tconstruct.melting.no_heat")); }
    if progress < 0 { return (182.0, 16.0, Some("gui.tconstruct.melting.no_space")); }
    (176.0, (16.0 * progress as f32 / required as f32).clamp(0.0, 16.0), None)
}

pub(super) fn draw_heat_bar(batch: &mut horizon_client::hud::HudBatch, atlas: &horizon_client::hud::HudAtlas,
    texture: &str, x: f32, y: f32, source_x: f32, height: f32) {
    if height <= 0.0 { return; }
    if let Some(mut sprite) = atlas.sprites.get(texture).copied() {
        let (du, dv) = ((sprite.u1 - sprite.u0) / sprite.w as f32, (sprite.v1 - sprite.v0) / sprite.h as f32);
        sprite.u0 += source_x * du;
        sprite.v0 += (150.0 + 16.0 - height) * dv;
        sprite.u1 = sprite.u0 + 3.0 * du; sprite.v1 = sprite.v0 + height * dv;
        batch.rect_uv(x - 4.0, y + 16.0 - height, 3.0, height, sprite, u32::MAX);
    }
}
