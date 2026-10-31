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
    classify_heat(recipe.map(|recipe| (recipe.temperature, recipe.time)), temperature, progress)
}

pub(super) fn classify_heat(recipe: Option<(u32, u32)>, temperature: u32, progress: i32) -> (f32, f32, Option<&'static str>) {
    let Some((recipe_temperature, time)) = recipe else { return (185.0, 16.0, Some("gui.tconstruct.melting.no_recipe")); };
    let required = time.saturating_mul(10);
    if required == 0 { return (185.0, 16.0, Some("gui.tconstruct.melting.no_recipe")); }
    if temperature < recipe_temperature { return (179.0, 16.0, Some("gui.tconstruct.melting.no_heat")); }
    if progress < 0 { return (182.0, 16.0, Some("gui.tconstruct.melting.no_space")); }
    (176.0, (16.0 * progress as f32 / required as f32).clamp(0.0, 16.0), None)
}

// Server-calculated status keeps remote GUI independent of client datapacks.
pub(super) fn encode_heat(recipe: Option<(u32, u32)>, temperature: u32, progress: i32) -> i32 {
    let (sprite, height, _) = classify_heat(recipe, temperature, progress);
    sprite as i32 * 32 + height.ceil() as i32
}
pub(super) fn decode_heat(code: i32) -> Option<(f32, f32, Option<&'static str>)> {
    let (sprite, height) = (code.div_euclid(32), code.rem_euclid(32));
    if height > 16 { return None; }
    let tip = match sprite { 176 => None, 179 => Some("gui.tconstruct.melting.no_heat"),
        182 => Some("gui.tconstruct.melting.no_space"), 185 => Some("gui.tconstruct.melting.no_recipe"), _ => return None };
    Some((sprite as f32, height as f32, tip))
}

pub(super) fn melter_heat(game: &super::Game<'_>, pos: (i32,i32,i32), slot: usize) -> Option<(f32, f32, Option<&'static str>)> {
    use horizon_world::nbt::Tag;
    let tag = super::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:melter")?;
    let Tag::IntArray(codes) = tag.get("heat_codes")? else { return None; };
    decode_heat(*codes.get(slot)?)
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

#[cfg(test)]
mod heat_tests {
    use super::{classify_heat, fire_height};
    #[test] fn source_heat_status_and_scaling() {
        assert_eq!(classify_heat(None, 500, 0).0, 185.0);
        assert_eq!(classify_heat(Some((600, 10)), 500, -1).0, 179.0);
        assert_eq!(classify_heat(Some((600, 10)), 600, -1).0, 182.0);
        assert_eq!(classify_heat(Some((600, 10)), 600, 50), (176.0, 8.0, None));
        assert_eq!((fire_height(50, 100), fire_height(1, 0), fire_height(u32::MAX, 1)), (7, 0, 14));
    }
}

/// GuiFuelModule uses the remaining fuel divided by its original quality.
pub(super) fn fire_height(fuel: u32, quality: u32) -> u32 {
    if quality == 0 { return 0; }
    ((14 * u64::from(fuel)) / u64::from(quality)).min(14) as u32
}
pub(super) fn draw_fire(batch: &mut horizon_client::hud::HudBatch, atlas: &horizon_client::hud::HudAtlas,
    texture: &str, x: f32, y: f32, fuel: u32, quality: u32) {
    let height = fire_height(fuel, quality) as f32;
    if height == 0.0 { return; }
    if let Some(mut sprite) = atlas.sprites.get(texture).copied() {
        let (du, dv) = ((sprite.u1 - sprite.u0) / sprite.w as f32, (sprite.v1 - sprite.v0) / sprite.h as f32);
        sprite.u0 += 176.0 * du; sprite.v0 += (136.0 + 14.0 - height) * dv;
        sprite.u1 = sprite.u0 + 14.0 * du; sprite.v1 = sprite.v0 + height * dv;
        batch.rect_uv(x, y + 14.0 - height, 14.0, height, sprite, u32::MAX);
    }
}

pub(super) fn fuel_tank(game: &super::Game<'_>, pos: (i32, i32, i32)) -> Option<super::tconstruct_source_melter_state::Tank> {
    let below = (pos.0, pos.1.checked_sub(1)?, pos.2);
    let name = game.res.facts().get(game.world.block(below.0, below.1, below.2)).name.clone();
    let capacity = super::tconstruct_source_smeltery_fuel::capacity(&name)?;
    let kind = super::tconstruct_source_machine_runtime::kind(&name)?;
    super::tconstruct_source_melter_store::read(&game.world, below, kind)
        .and_then(|tag| super::tconstruct_source_melter_state::Tank::from_nbt(&tag, capacity))
}
