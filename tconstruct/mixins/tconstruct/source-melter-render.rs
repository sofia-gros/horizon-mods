// Vanilla fluids use their existing still sprites; TConstruct fluids use the
// source images staged under a separate prefix.
pub(super) fn sprite(fluid: &str) -> String {
    match fluid {
        "minecraft:lava" => "block/lava_still".to_owned(),
        "minecraft:water" => "block/water_still".to_owned(),
        _ => format!("block/tconstruct_fluid_{}", fluid.strip_prefix("tconstruct:").unwrap_or("")),
    }
}

pub(super) fn fluid_name(game: &super::Game<'_>, fluid: &str) -> String {
    let key = format!("fluid.{}", fluid.replace(':', "."));
    game.pack.names.display_get(&key).or_else(|| {
        fluid.strip_prefix("minecraft:").and_then(|id| game.pack.names.display_get(&format!("block.minecraft.{id}")))
    }).unwrap_or_else(|| match fluid { "minecraft:lava" => "Lava".to_owned(), "minecraft:water" => "Water".to_owned(), _ => fluid.to_owned() })
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

/// Potion NBT uses RGB; the renderer stores packed colors in ABGR order.
pub(super) fn color_tagged(fluid: &str, tag: Option<&horizon_world::nbt::Tag>) -> u32 {
    if fluid == "tconstruct:potion" {
        if let Some(horizon_world::nbt::Tag::Int(color)) = tag.and_then(|tag| tag.get("CustomPotionColor")) {
            let rgb = *color as u32;
            return 0xff00_0000 | (rgb & 0xff) << 16 | (rgb & 0xff00) | (rgb >> 16 & 0xff);
        }
    }
    color(fluid)
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
    use super::{classify_heat, decode_heat, encode_heat, fire_height};
    #[test] fn source_heat_status_and_scaling() {
        assert_eq!(classify_heat(None, 500, 0).0, 185.0);
        assert_eq!(classify_heat(Some((600, 10)), 500, -1).0, 179.0);
        assert_eq!(classify_heat(Some((600, 10)), 600, -1).0, 182.0);
        assert_eq!(classify_heat(Some((600, 10)), 600, 50), (176.0, 8.0, None));
        assert_eq!((fire_height(50, 100), fire_height(1, 0), fire_height(u32::MAX, 1)), (7, 0, 14));
        assert_eq!(decode_heat(encode_heat(Some((600, 10)), 600, 50)), Some((176.0, 8.0, None)));
        assert_eq!(decode_heat(encode_heat(None, 0, 0)).unwrap().2, Some("gui.tconstruct.melting.no_recipe"));
        assert_eq!(decode_heat(-1), None);
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

/// Horizontal faucet model at y=0 is attached to the south face and opens
/// towards the centre. Rotate its stream using the blockstate model rotations.
pub(super) fn faucet_stream_bounds(facing: &str) -> [[f32; 6]; 2] {
    [[6.0, 6.0, 10.0, 10.0, 9.0, 16.0], [6.0, 0.0, 8.0, 10.0, 9.0, 10.0]].map(|[x0,y0,z0,x1,y1,z1]| {
        match facing {
            "south" => [16.0-x1,y0,16.0-z1,16.0-x0,y1,16.0-z0],
            "east" => [16.0-z1,y0,x0,16.0-z0,y1,x1],
            "west" => [z0,y0,16.0-x1,z1,y1,16.0-x0],
            _ => [x0,y0,z0,x1,y1,z1],
        }
    })
}

#[cfg(test)]
mod faucet_geometry_tests {
    use super::faucet_stream_bounds;
    #[test] fn stream_starts_at_attached_side_and_ends_at_outlet() {
        assert_eq!(faucet_stream_bounds("north"), [[6.,6.,10.,10.,9.,16.],[6.,0.,8.,10.,9.,10.]]);
        assert_eq!(faucet_stream_bounds("south"), [[6.,6.,0.,10.,9.,6.],[6.,0.,6.,10.,9.,8.]]);
        assert_eq!(faucet_stream_bounds("east"), [[0.,6.,6.,6.,9.,10.],[6.,0.,6.,8.,9.,10.]]);
        assert_eq!(faucet_stream_bounds("west"), [[10.,6.,6.,16.,9.,10.],[8.,0.,6.,10.,9.,10.]]);
    }
}

/// Source TinkerFluids registrations provide the minimum block light used by
/// Mantle's fluid renderer. Unknown fluids retain the world's existing light.
pub(super) fn fluid_luminosity(fluid: &str) -> u8 {
    match fluid {
        "minecraft:lava" | "tconstruct:fiery_liquid" => 15,
        "tconstruct:molten_glass" => 1,
        "tconstruct:liquid_soul" | "tconstruct:molten_porcelain" => 2,
        "tconstruct:magma" | "tconstruct:molten_clay" | "tconstruct:molten_obsidian" => 3,
        "tconstruct:scorched_stone" | "tconstruct:molten_osmium" => 4,
        "tconstruct:molten_ender" => 5,
        "tconstruct:seared_stone" | "tconstruct:molten_quartz" | "tconstruct:molten_soulsteel" => 6,
        "tconstruct:molten_refined_obsidian" => 7,
        "tconstruct:molten_cobalt" | "tconstruct:molten_hepatizon" => 8,
        "tconstruct:molten_emerald" | "tconstruct:molten_queens_slime" | "tconstruct:molten_bendalloy" => 9,
        "tconstruct:molten_slimesteel" | "tconstruct:molten_pig_iron" | "tconstruct:molten_bronze" | "tconstruct:molten_brass" | "tconstruct:molten_electrum" | "tconstruct:molten_invar" | "tconstruct:molten_constantan" | "tconstruct:molten_pewter" | "tconstruct:molten_cinderslime" | "tconstruct:molten_knightmetal" | "tconstruct:molten_cadmium" | "tconstruct:molten_duralumin" | "tconstruct:molten_steeleaf" => 10,
        "tconstruct:molten_amethyst" | "tconstruct:molten_manyullyn" => 11,
        "tconstruct:molten_iron" | "tconstruct:molten_gold" | "tconstruct:molten_copper" | "tconstruct:molten_amethyst_bronze" | "tconstruct:molten_rose_gold" | "tconstruct:molten_knightslime" | "tconstruct:molten_tin" | "tconstruct:molten_aluminum" | "tconstruct:molten_lead" | "tconstruct:molten_silver" | "tconstruct:molten_nickel" | "tconstruct:molten_zinc" | "tconstruct:molten_platinum" | "tconstruct:molten_tungsten" | "tconstruct:molten_enderium" => 12,
        "tconstruct:molten_diamond" | "tconstruct:molten_steel" | "tconstruct:molten_signalum" | "tconstruct:molten_chromium" => 13,
        "tconstruct:molten_debris" | "tconstruct:molten_netherite" | "tconstruct:molten_nicrosil" => 14,
        "tconstruct:blazing_blood" | "tconstruct:molten_uranium" | "tconstruct:molten_lumium" | "tconstruct:molten_refined_glowstone" => 15,
        _ => 0,
    }
}

/// Horizon packs sky into the low byte and block light into the next byte.
pub(super) fn fluid_light(world_light: u8, fluid: &str) -> u32 {
    let sky = u32::from(world_light >> 4) * 16;
    let block = u32::from((world_light & 15).max(fluid_luminosity(fluid))) * 16;
    sky | (block << 8)
}

#[cfg(test)]
mod fluid_light_tests {
    use super::{fluid_light, fluid_luminosity};
    #[test] fn source_emission_preserves_sky_and_brighter_world_light() {
        for (id, expected) in [
            ("tconstruct:molten_cinderslime", 10), ("tconstruct:molten_knightmetal", 10),
            ("tconstruct:molten_cadmium", 10), ("tconstruct:molten_duralumin", 10),
            ("tconstruct:molten_steeleaf", 10), ("tconstruct:molten_chromium", 13),
            ("tconstruct:molten_nicrosil", 14), ("tconstruct:molten_bendalloy", 9),
            ("tconstruct:fiery_liquid", 15),
        ] { assert_eq!(fluid_luminosity(id), expected, "{id}"); }
        assert_eq!(fluid_luminosity("other:molten_cinderslime"), 0);
        assert_eq!(fluid_light(0x20, "tconstruct:molten_copper"), 32 | (192 << 8));
        assert_eq!(fluid_light(0xff, "tconstruct:molten_copper"), 240 | (240 << 8));
        assert_eq!(fluid_light(0x52, "minecraft:water"), 80 | (32 << 8));
        assert_eq!(fluid_light(0x00, "minecraft:lava"), 240 << 8);
    }
}
