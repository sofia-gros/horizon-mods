use horizon_world::nbt::Tag;
use horizon_items::items::ItemStack;
use super::tconstruct_source_material_traits::Traits;

fn temperature(game: &super::Game<'_>) -> Option<f32> {
    let position=game.player.as_ref()?.position;
    if ![position.x,position.y,position.z].iter().all(|v| v.is_finite()
        && v.floor() >= i32::MIN as f64 && v.floor() < i32::MAX as f64) { return None; }
    let pos=horizon_core::position::BlockPos::new(position.x.floor() as i32,position.y.floor() as i32,position.z.floor() as i32);
    // Level.getBiome uses the seeded eight-corner zoom, not the raw cell underfoot.
    let [qx,qy,qz]=game.streaming.biome_zoom.quart_at_block(pos);
    let [x,y,z]=[qx << 2,qy << 2,qz << 2];
    let chunk=game.world.chunks.get(&(x >> 4,z >> 4))?;
    let biomes=match &game.world_biomes { Some(biomes)=>biomes, None=>game.res.biomes() };
    let biome=biomes.list.get(usize::from(chunk.biome(x,y,z)))?;
    // Source getTemperature(BlockPos) starts height cooling above Y=80.
    // The host expresses that same offset as sea_level + 17.
    let value=biome.temperature_at(pos,63);
    value.is_finite().then_some(value)
}

fn eligible(game: &super::Game<'_>, stack: &ItemStack, traits: &Traits) -> bool {
    if !super::tconstruct_source_flamestance::supported_definition(&game.world)
        || traits.levels.get("tconstruct:flamestance").copied().unwrap_or(0) <= 0 { return false; }
    let Some(custom)=stack.data().and_then(|data| data.other("minecraft:custom_data")) else { return false; };
    !custom.get("tic_upgrades").is_some_and(|upgrades| !matches!(upgrades,Tag::Compound(_))
        || upgrades.get("tconstruct:flamestance").is_some())
}

pub(super) fn melee_bonus(game: &super::Game<'_>, stack: &ItemStack, traits: &Traits, multiplier: f32) -> Option<f32> {
    if !eligible(game,stack,traits) { return None; }
    super::tconstruct_source_flamestance::bonus(temperature(game)?,multiplier,false)
}

pub(super) fn projectile_bonus(game: &super::Game<'_>, stack: &ItemStack) -> Option<f32> {
    if game.remote.is_some() || game.held() != Some(stack) || stack.count != 1 || stack.wire_patch().is_some()
        || !matches!(stack.item.as_ref(),"tconstruct:longbow" | "tconstruct:crossbow")
        || super::tconstruct_source_melee_shape::intact(stack).is_none() { return None; }
    let custom=stack.data()?.other("minecraft:custom_data")?;
    if !matches!(custom.get("tic_broken"),None | Some(Tag::Byte(0))) { return None; }
    let tags=game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    if !tags.get("tconstruct:modifiable/ranged/launcher")?.contains(stack.item.as_ref()) { return None; }
    let traits=super::tconstruct_source_material_traits::levels(&game.world,stack)?;
    if !eligible(game,stack,&traits) { return None; }
    let multiplier=match custom.get("tic_raging_projectile_multiplier") {
        Some(Tag::Float(bits))=>f32::from_bits(*bits), _=>return None,
    };
    super::tconstruct_source_flamestance::bonus(temperature(game)?,multiplier,true)
}
