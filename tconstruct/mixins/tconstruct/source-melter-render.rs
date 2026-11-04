// Vanilla fluids use their existing still sprites; TConstruct fluids use the
// source images staged under a separate prefix.
pub(super) fn sprite(fluid: &str) -> String {
    super::tconstruct_source_fluid_display::sprite(fluid)
}

pub(super) fn fluid_name(game: &super::Game<'_>, fluid: &str) -> String {
    let fluid = super::tconstruct_source_fluid_display::display_id(fluid);
    if fluid == "minecraft:milk" {
        return game.pack.names.display_get("fluid_type.minecraft.milk").unwrap_or_else(|| "Milk".to_owned());
    }
    let key = format!("fluid.{}", fluid.replace(':', "."));
    game.pack.names.display_get(&key).or_else(|| {
        fluid.strip_prefix("minecraft:").and_then(|id| game.pack.names.display_get(&format!("block.minecraft.{id}")))
    }).unwrap_or_else(|| match fluid { "minecraft:lava" => "Lava".to_owned(), "minecraft:water" => "Water".to_owned(), "minecraft:milk" => "Milk".to_owned(), _ => fluid.to_owned() })
}

/// Resolve potion names only when the caller knows the fluid metadata is complete.
/// Remote machine snapshots may omit tags, so they must retain the generic name.
pub(super) fn fluid_name_tagged(game: &super::Game<'_>, fluid: &str, tag: Option<&horizon_world::nbt::Tag>, metadata_complete: bool) -> String {
    if fluid == "tconstruct:potion" && metadata_complete {
        if let Some(name) = super::tconstruct_source_smeltery_fuel::potion_fluid_name(tag)
            .and_then(|key| game.pack.names.display_get(&key)) { return name; }
    }
    fluid_name(game, fluid)
}

// Source FluidTextureProvider colors plus Forge's default vanilla water tint.
pub(super) fn color(fluid: &str) -> u32 {
    match super::tconstruct_source_fluid_display::display_id(fluid) {
        // Forge FluidStack water tint: ARGB 0xff3f76e4, packed as ABGR.
        "minecraft:water" => 0xffe4763f,
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

/// Only locally complete machine metadata can resolve potion effects. Remote
/// snapshots may omit fields, so they retain the previous conservative tint.
pub(super) fn color_with_complete(fluid: &str, tag: Option<&horizon_world::nbt::Tag>, metadata_complete: bool) -> u32 {
    if fluid == "tconstruct:potion" && metadata_complete {
        if let Some(color) = tag.and_then(can_potion_color) { return color; }
    }
    color_tagged(fluid, tag)
}

/// Source 1.20.1 PotionUtils uses float32 weighted sums, not the host's
/// integer average. Refuse oversized or unsupported effect lists.
pub(super) fn can_potion_color(tag: &horizon_world::nbt::Tag) -> Option<u32> {
    use horizon_world::nbt::Tag;
    if let Some(color) = super::tconstruct_source_fluid_container::display_potion_color(tag) {
        let rgb = color as u32;
        return Some(0xff00_0000 | (rgb & 0xff) << 16 | (rgb & 0xff00) | (rgb >> 16 & 0xff));
    }
    let (id, _, _) = super::tconstruct_source_fluid_container::potion_tag(tag, true)?;
    super::tconstruct_source_smeltery_fuel::potion_fluid_name(Some(tag))?;
    let base = horizon_client_core::potions::potion(&id)?;
    let custom = match tag.get("CustomPotionEffects") {
        None => &[][..],
        Some(Tag::List(effects)) if effects.len() <= 64 => effects.as_slice(),
        _ => return None,
    };
    let total_effects = base.effects.len() + custom.len();
    if total_effects == 0 { return Some(0xffc65d38); }
    let (mut r, mut g, mut b, mut total) = (0.0f32, 0.0f32, 0.0f32, 0i32);
    let mut add = |effect: &str, amplifier: i32, visible: bool| -> Option<()> {
        if visible {
            let color = horizon_client_core::potions::effect(effect)?.color;
            let weight = amplifier.max(0) + 1;
            r += (weight * ((color >> 16 & 255) as i32)) as f32 / 255.0f32;
            g += (weight * ((color >> 8 & 255) as i32)) as f32 / 255.0f32;
            b += (weight * ((color & 255) as i32)) as f32 / 255.0f32;
            total += weight;
        }
        Some(())
    };
    for &(effect, _, amplifier) in base.effects { add(effect, amplifier, true)?; }
    for effect in custom {
        let Tag::Byte(id) = effect.get("Id")? else { return None; };
        let effect_id = *super::tconstruct_source_fluid_container::EFFECT_IDS.get((*id as usize).checked_sub(1)?)?;
        let amplifier = match effect.get("Amplifier") { None => 0, Some(Tag::Byte(value)) => i32::from(*value).max(0), _ => return None };
        let visible = match effect.get("ShowParticles") { None | Some(Tag::Byte(1)) => true, Some(Tag::Byte(0)) => false, _ => return None };
        add(effect_id, amplifier, visible)?;
    }
    if total == 0 { return Some(0xff000000); }
    let (r, g, b) = ((r / total as f32 * 255.0f32) as u32,
                     (g / total as f32 * 255.0f32) as u32,
                     (b / total as f32 * 255.0f32) as u32);
    Some(0xff00_0000 | b << 16 | g << 8 | r)
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
    match super::tconstruct_source_fluid_display::display_id(fluid) {
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
    // 1.20.1-labelled PotionUtils/Potions/MobEffects oracle, decompiled
    // source commit 315a03ea8861509b550cc06a7ca6b28853d2459c; not an official jar probe.
    #[test] fn all_source_base_potion_colors_match() {
        use horizon_world::nbt::Tag;
        for (id, expected) in [
            ("minecraft:water", 0xffc65d38),
            ("minecraft:mundane", 0xffc65d38),
            ("minecraft:thick", 0xffc65d38),
            ("minecraft:awkward", 0xffc65d38),
            ("minecraft:night_vision", 0xff66ffc2),
            ("minecraft:long_night_vision", 0xff66ffc2),
            ("minecraft:invisibility", 0xfff6f6f6),
            ("minecraft:long_invisibility", 0xfff6f6f6),
            ("minecraft:leaping", 0xff84fffd),
            ("minecraft:long_leaping", 0xff84fffd),
            ("minecraft:strong_leaping", 0xff84fffd),
            ("minecraft:fire_resistance", 0xff0099ff),
            ("minecraft:long_fire_resistance", 0xff0099ff),
            ("minecraft:swiftness", 0xffffeb33),
            ("minecraft:long_swiftness", 0xffffeb33),
            ("minecraft:strong_swiftness", 0xffffeb33),
            ("minecraft:slowness", 0xffe0af8b),
            ("minecraft:long_slowness", 0xffe0af8b),
            ("minecraft:strong_slowness", 0xffe0af8b),
            ("minecraft:turtle_master", 0xffe6828d),
            ("minecraft:long_turtle_master", 0xffe6828d),
            ("minecraft:strong_turtle_master", 0xffe6858d),
            ("minecraft:water_breathing", 0xffc0da98),
            ("minecraft:long_water_breathing", 0xffc0da98),
            ("minecraft:healing", 0xff2324f8),
            ("minecraft:strong_healing", 0xff2324f8),
            ("minecraft:harming", 0xff6a65a9),
            ("minecraft:strong_harming", 0xff6a65a9),
            ("minecraft:poison", 0xff63a387),
            ("minecraft:long_poison", 0xff63a387),
            ("minecraft:strong_poison", 0xff63a387),
            ("minecraft:regeneration", 0xffab5ccd),
            ("minecraft:long_regeneration", 0xffab5ccd),
            ("minecraft:strong_regeneration", 0xffab5ccd),
            ("minecraft:strength", 0xff00c7ff),
            ("minecraft:long_strength", 0xff00c7ff),
            ("minecraft:strong_strength", 0xff00c7ff),
            ("minecraft:weakness", 0xff484d48),
            ("minecraft:long_weakness", 0xff484d48),
            ("minecraft:luck", 0xff06c159),
            ("minecraft:slow_falling", 0xffb9cff3),
            ("minecraft:long_slow_falling", 0xffb9cff3),
        ] {
            let tag = Tag::compound([("Potion", Tag::String(id.into()))]);
            assert_eq!(super::can_potion_color(&tag), Some(expected), "{id}");
        }
    }
    // Source-order 1.20.1-labelled decompiled oracle at commit 315a03ea;
    // these do not replace an official jar or integrated rendering test.
    // OpenJDK 21 evaluation of 1.20.1-labelled decompiled Mth.floor at
    // commit 315a03ea, not an official-jar probe. Retain source boundary bits.
    #[test] fn floating_color_source_boundary_oracle() {
        use horizon_world::nbt::Tag;
        for (value, integer, color) in [
            (Tag::Float(0x00000000u32), 0i32, 0xff000000u32),
            (Tag::Float(0x80000000u32), 0i32, 0xff000000u32),
            (Tag::Float(0x00000001u32), 0i32, 0xff000000u32),
            (Tag::Float(0x80000001u32), -1i32, 0xffffffffu32),
            (Tag::Float(0x3e800000u32), 0i32, 0xff000000u32),
            (Tag::Float(0xbe800000u32), -1i32, 0xffffffffu32),
            (Tag::Float(0x3fc00000u32), 1i32, 0xff010000u32),
            (Tag::Float(0xbfc00000u32), -2i32, 0xfffeffffu32),
            (Tag::Float(0x4affffffu32), 8388607i32, 0xffffff7fu32),
            (Tag::Float(0x4b000000u32), 8388608i32, 0xff000080u32),
            (Tag::Float(0x4b7fffffu32), 16777215i32, 0xffffffffu32),
            (Tag::Float(0x4b800000u32), 16777216i32, 0xff000000u32),
            (Tag::Float(0x4effffffu32), 2147483520i32, 0xff80ffffu32),
            (Tag::Float(0x4f000000u32), 2147483647i32, 0xffffffffu32),
            (Tag::Float(0x4f000001u32), 2147483647i32, 0xffffffffu32),
            (Tag::Float(0xceffffffu32), -2147483520i32, 0xff800000u32),
            (Tag::Float(0xcf000000u32), -2147483648i32, 0xff000000u32),
            (Tag::Float(0xcf000001u32), 2147483647i32, 0xffffffffu32),
            (Tag::Float(0x7f7fffffu32), 2147483647i32, 0xffffffffu32),
            (Tag::Float(0xff7fffffu32), 2147483647i32, 0xffffffffu32),
            (Tag::Float(0x7f800000u32), 2147483647i32, 0xffffffffu32),
            (Tag::Float(0xff800000u32), 2147483647i32, 0xffffffffu32),
            (Tag::Float(0x7fc00000u32), 0i32, 0xff000000u32),
            (Tag::Float(0xffc00000u32), 0i32, 0xff000000u32),
            (Tag::Float(0x7f800001u32), 0i32, 0xff000000u32),
            (Tag::Double(0x0000000000000000u64), 0i32, 0xff000000u32),
            (Tag::Double(0x8000000000000000u64), 0i32, 0xff000000u32),
            (Tag::Double(0x0000000000000001u64), 0i32, 0xff000000u32),
            (Tag::Double(0x8000000000000001u64), -1i32, 0xffffffffu32),
            (Tag::Double(0x3fd0000000000000u64), 0i32, 0xff000000u32),
            (Tag::Double(0xbfd0000000000000u64), -1i32, 0xffffffffu32),
            (Tag::Double(0x3ff8000000000000u64), 1i32, 0xff010000u32),
            (Tag::Double(0xbff8000000000000u64), -2i32, 0xfffeffffu32),
            (Tag::Double(0x416fffffe0000000u64), 16777215i32, 0xffffffffu32),
            (Tag::Double(0x4170000000000000u64), 16777216i32, 0xff000000u32),
            (Tag::Double(0x41dfffffffc00000u64), 2147483647i32, 0xffffffffu32),
            (Tag::Double(0x41dfffffffe00000u64), 2147483647i32, 0xffffffffu32),
            (Tag::Double(0x41dfffffffffffffu64), 2147483647i32, 0xffffffffu32),
            (Tag::Double(0x41e0000000000000u64), 2147483647i32, 0xffffffffu32),
            (Tag::Double(0x41e0000000000001u64), 2147483647i32, 0xffffffffu32),
            (Tag::Double(0xc1dfffffffffffffu64), -2147483648i32, 0xff000000u32),
            (Tag::Double(0xc1e0000000000000u64), -2147483648i32, 0xff000000u32),
            (Tag::Double(0xc1e0000000000001u64), 2147483647i32, 0xffffffffu32),
            (Tag::Double(0x7fefffffffffffffu64), 2147483647i32, 0xffffffffu32),
            (Tag::Double(0xffefffffffffffffu64), 2147483647i32, 0xffffffffu32),
            (Tag::Double(0x7ff0000000000000u64), 2147483647i32, 0xffffffffu32),
            (Tag::Double(0xfff0000000000000u64), 2147483647i32, 0xffffffffu32),
            (Tag::Double(0x7ff8000000000000u64), 0i32, 0xff000000u32),
            (Tag::Double(0xfff8000000000000u64), 0i32, 0xff000000u32),
            (Tag::Double(0x7ff0000000000001u64), 0i32, 0xff000000u32),
        ] {
            let tag = Tag::compound([("CustomPotionColor", value)]);
            assert_eq!(super::super::tconstruct_source_fluid_container::display_potion_color(&tag), Some(integer));
            assert_eq!(super::can_potion_color(&tag), Some(color));
        }
    }
    #[test] fn floating_nbt_potion_colors_and_guest_boundary() {
        use horizon_world::nbt::Tag;
        for (value, expected) in [(Tag::Float((-1.25f32).to_bits()), 0xfffeffff),
            (Tag::Double(1193046.75f64.to_bits()), 0xff563412),
            (Tag::Double(f64::NEG_INFINITY.to_bits()), 0xffffffff),
            (Tag::Float(f32::NAN.to_bits()), 0xff000000)] {
            let tag = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into())), ("CustomPotionColor", value)]);
            assert_eq!(super::can_potion_color(&tag), Some(expected));
            assert_eq!(super::color_with_complete("tconstruct:potion", Some(&tag), true), expected);
            assert_eq!(super::color_with_complete("tconstruct:potion", Some(&tag), false), 0xfff800f8);
        }
    }
    #[test] fn integer_nbt_potion_colors_use_source_narrowing() {
        use horizon_world::nbt::Tag;
        for (value, expected) in [(Tag::Byte(-128), 0xff80ffff), (Tag::Byte(127), 0xff7f0000),
            (Tag::Short(-32768), 0xff0080ff), (Tag::Short(32767), 0xffff7f00),
            (Tag::Long(0x1234_5678_9abc_def0), 0xfff0debc), (Tag::Long(-1), 0xffffffff)] {
            let tag = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into())), ("CustomPotionColor", value)]);
            assert_eq!(super::can_potion_color(&tag), Some(expected));
            assert_eq!(super::color_with_complete("tconstruct:potion", Some(&tag), true), expected);
            assert_eq!(super::color_with_complete("tconstruct:potion", Some(&tag), false), 0xfff800f8);
        }
    }
    #[test] fn potion_machine_color_requires_complete_metadata() {
        use horizon_world::nbt::Tag;
        let base = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into()))]);
        assert_eq!(super::color_with_complete("tconstruct:potion", Some(&base), true), 0xffffeb33);
        assert_eq!(super::color_with_complete("tconstruct:potion", Some(&base), false), 0xfff800f8);
        let mixed = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(1)), ("Amplifier", Tag::Byte(4))])]))]);
        assert_eq!(super::color_with_complete("tconstruct:potion", Some(&mixed), true), 0xffffea33);
        assert_eq!(super::color_with_complete("tconstruct:potion", Some(&mixed), false), 0xfff800f8);
        assert_eq!(super::color_with_complete("tconstruct:potion", None, true), 0xfff800f8);
        let explicit = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into())), ("CustomPotionColor", Tag::Int(0x123456))]);
        assert_eq!(super::color_with_complete("tconstruct:potion", Some(&explicit), false), 0xff563412);
        let unknown = Tag::compound([("Potion", Tag::String("example:unknown".into()))]);
        assert_eq!(super::color_with_complete("tconstruct:potion", Some(&unknown), true), 0xfff800f8);
        assert_eq!(super::color_with_complete("minecraft:water", None, true), 0xffe4763f);
    }
    #[test] fn source_custom_potion_effect_mixtures() {
        use horizon_world::nbt::Tag;
        for (name, tag, expected) in [
            ("empty_custom_list", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![]))]), 0xffc65d38),
            ("nonempty_all_hidden", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(1)), ("ShowParticles", Tag::Byte(0))])]))]), 0xff000000),
            ("default_amplifier_visibility", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(1))])]))]), 0xffffeb33),
            ("negative_amplifier", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(1)), ("Amplifier", Tag::Byte(-1))])]))]), 0xffffeb33),
            ("minimum_amplifier", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(1)), ("Amplifier", Tag::Byte(-128))])]))]), 0xffffeb33),
            ("f32_rounding_diff", Tag::compound([("Potion", Tag::String("minecraft:swiftness".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(1)), ("Amplifier", Tag::Byte(4))])]))]), 0xffffea33),
            ("zero_duration_still_colored", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(1)), ("Duration", Tag::Int(0)), ("ShowIcon", Tag::Byte(0)), ("Ambient", Tag::Byte(1))])]))]), 0xffffeb33),
            ("negative_duration_still_colored", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(1)), ("Duration", Tag::Int(-100))])]))]), 0xffffeb33),
            ("hidden_effect_not_mixed", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(1)), ("HiddenEffect", Tag::compound([("Id", Tag::Byte(2)), ("Amplifier", Tag::Byte(127))]))])]))]), 0xffffeb33),
            ("hidden_main_with_visible_child", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(1)), ("ShowParticles", Tag::Byte(0)), ("HiddenEffect", Tag::compound([("Id", Tag::Byte(2)), ("ShowParticles", Tag::Byte(1))]))])]))]), 0xff000000),
            ("turtle_with_speed", Tag::compound([("Potion", Tag::String("minecraft:turtle_master".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(1)), ("Amplifier", Tag::Byte(4))])]))]), 0xfff0ad67),
            ("strong_turtle_with_poison", Tag::compound([("Potion", Tag::String("minecraft:strong_turtle_master".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(19)), ("Amplifier", Tag::Byte(127))])]))]), 0xff6ca087),
            ("sixty_four_speed_effects", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(1)), ("Amplifier", Tag::Byte(127))]); 64]))]), 0xffffeb32),
            ("effect_speed", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(1))])]))]), 0xffffeb33),
            ("effect_slowness", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(2))])]))]), 0xffe0af8b),
            ("effect_haste", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(3))])]))]), 0xff43c0d9),
            ("effect_mining_fatigue", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(4))])]))]), 0xff17424a),
            ("effect_strength", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(5))])]))]), 0xff00c7ff),
            ("effect_instant_health", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(6))])]))]), 0xff2324f8),
            ("effect_instant_damage", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(7))])]))]), 0xff6a65a9),
            ("effect_jump_boost", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(8))])]))]), 0xff84fffd),
            ("effect_nausea", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(9))])]))]), 0xff4a1d55),
            ("effect_regeneration", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(10))])]))]), 0xffab5ccd),
            ("effect_resistance", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(11))])]))]), 0xfff04691),
            ("effect_fire_resistance", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(12))])]))]), 0xff0099ff),
            ("effect_water_breathing", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(13))])]))]), 0xffc0da98),
            ("effect_invisibility", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(14))])]))]), 0xfff6f6f6),
            ("effect_blindness", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(15))])]))]), 0xff231f1f),
            ("effect_night_vision", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(16))])]))]), 0xff66ffc2),
            ("effect_hunger", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(17))])]))]), 0xff537658),
            ("effect_weakness", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(18))])]))]), 0xff484d48),
            ("effect_poison", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(19))])]))]), 0xff63a387),
            ("effect_wither", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(20))])]))]), 0xff566173),
            ("effect_health_boost", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(21))])]))]), 0xff237df8),
            ("effect_absorption", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(22))])]))]), 0xffa55225),
            ("effect_saturation", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(23))])]))]), 0xff2324f8),
            ("effect_glowing", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(24))])]))]), 0xff61a094),
            ("effect_levitation", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(25))])]))]), 0xffffffce),
            ("effect_luck", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(26))])]))]), 0xff06c159),
            ("effect_unluck", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(27))])]))]), 0xff4da4c0),
            ("effect_slow_falling", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(28))])]))]), 0xffb9cff3),
            ("effect_conduit_power", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(29))])]))]), 0xffd1c21d),
            ("effect_dolphins_grace", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(30))])]))]), 0xffbea388),
            ("effect_bad_omen", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(31))])]))]), 0xff38610b),
            ("effect_hero_of_the_village", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(32))])]))]), 0xff44ff44),
            ("effect_darkness", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(33))])]))]), 0xff212729),
            ("override_custom_mix", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(1)), ("ShowParticles", Tag::Byte(1)), ("Amplifier", Tag::Byte(4))]), Tag::compound([("Id", Tag::Byte(2)), ("Amplifier", Tag::Byte(127))])])), ("CustomPotionColor", Tag::Int(1193046))]), 0xff563412),
            ("override_all_hidden", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(1)), ("ShowParticles", Tag::Byte(0)), ("Amplifier", Tag::Byte(4))]), Tag::compound([("Id", Tag::Byte(2)), ("Amplifier", Tag::Byte(127))])])), ("CustomPotionColor", Tag::Int(0))]), 0xff000000),
            ("override_negative", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(1)), ("ShowParticles", Tag::Byte(1)), ("Amplifier", Tag::Byte(4))]), Tag::compound([("Id", Tag::Byte(2)), ("Amplifier", Tag::Byte(127))])])), ("CustomPotionColor", Tag::Int(-1))]), 0xffffffff),
            ("override_int_min", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(1)), ("ShowParticles", Tag::Byte(1)), ("Amplifier", Tag::Byte(4))]), Tag::compound([("Id", Tag::Byte(2)), ("Amplifier", Tag::Byte(127))])])), ("CustomPotionColor", Tag::Int(-2147483648))]), 0xff000000),
            ("override_high_bits", Tag::compound([("Potion", Tag::String("minecraft:water".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(1)), ("ShowParticles", Tag::Byte(1)), ("Amplifier", Tag::Byte(4))]), Tag::compound([("Id", Tag::Byte(2)), ("Amplifier", Tag::Byte(127))])])), ("CustomPotionColor", Tag::Int(16909060))]), 0xff040302),
        ] {
            assert_eq!(super::can_potion_color(&tag), Some(expected), "{name}");
        }
        let oversized = Tag::compound([("Potion", Tag::String("minecraft:water".into())),
            ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(1))]); 65]))]);
        assert_eq!(super::can_potion_color(&oversized), None);
    }
    #[test] fn copper_can_potion_base_colors_and_extra_effect_boundary() {
        use horizon_world::nbt::Tag;
        for (id, expected) in [("minecraft:water", 0xffc65d38), ("minecraft:swiftness", 0xffffeb33), ("minecraft:turtle_master", 0xffe6828d)] {
            let tag = Tag::compound([("Potion", Tag::String(id.into()))]);
            assert_eq!(super::can_potion_color(&tag), Some(expected), "{id}");
        }
        let effects = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into())), ("CustomPotionEffects", Tag::List(vec![]))]);
        assert_eq!(super::can_potion_color(&effects), Some(0xffffeb33));
        let explicit = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into())), ("CustomPotionEffects", Tag::List(vec![])), ("CustomPotionColor", Tag::Int(0x123456))]);
        assert_eq!(super::can_potion_color(&explicit), Some(0xff563412));
        let unknown = Tag::compound([("Potion", Tag::String("example:unknown".into()))]);
        assert_eq!(super::can_potion_color(&unknown), None);
    }
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
        assert_eq!(fluid_light(0x00, "minecraft:flowing_lava"), fluid_light(0x00, "minecraft:lava"));
        assert_eq!(fluid_light(0x52, "minecraft:flowing_water"), fluid_light(0x52, "minecraft:water"));
        assert_eq!(fluid_light(0xff, "minecraft:flowing_lava"), fluid_light(0xff, "minecraft:lava"));
        assert_eq!(fluid_light(0x00, "tconstruct:flowing_molten_copper"), fluid_light(0x00, "tconstruct:molten_copper"));
        assert_eq!(super::color("tconstruct:flowing_molten_clay"), super::color("tconstruct:molten_clay"));
        assert_eq!(super::color("minecraft:water"), 0xffe4763f);
        assert_eq!(super::color("minecraft:flowing_water"), 0xffe4763f);
        assert_eq!(super::color_tagged("minecraft:water", None), 0xffe4763f);
        assert_eq!(super::color("minecraft:lava"), u32::MAX);
    }
}
