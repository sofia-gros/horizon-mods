use super::tconstruct_world::World;

pub(super) fn supported_definition(world: &World) -> bool {
    supported_target_definition(world, "smite", "undead")
}

pub(super) fn supported_antiaquatic_definition(world: &World) -> bool {
    supported_target_definition(world, "antiaquatic", "water")
}

fn supported_target_definition(world: &World, id: &str, mob_type: &str) -> bool {
    let Some(raw) = world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()
        .and_then(|raw| raw.as_ref().ok()) else { return false; };
    let Some(bytes) = raw.0.get(&format!("tconstruct:tinkering/modifiers/{id}")).and_then(|layers| layers.last()) else { return false; };
    serde_json::from_slice::<serde_json::Value>(bytes).ok().is_some_and(|value|
        value == serde_json::json!({"level_display":"tconstruct:default", "modules":[{
            "type":"tconstruct:conditional_melee_damage", "each_level":2.0, "percent":false,
            "target":{"type":"mantle:mob_type", "mobs":format!("minecraft:{mob_type}")}
        }]}))
}

/// Java MobType.WATER is a type classification, not wetness or swimming.
/// Use the twelve vanilla source types; custom/newer entities remain closed.
pub(super) fn water_mob(id: &str) -> bool {
    matches!(id.strip_prefix("minecraft:").unwrap_or(id),
        "cod" | "salmon" | "pufferfish" | "tropical_fish" | "tadpole"
        | "dolphin" | "squid" | "glow_squid" | "guardian" | "elder_guardian"
        | "axolotl" | "turtle")
}

pub(super) fn effective_level(trait_level: i32, amount: i32) -> Option<f32> {
    effective_level_with_needed(trait_level, amount, 5)
}

pub(super) fn effective_level_with_needed(trait_level: i32, amount: i32, needed: i32) -> Option<f32> {
    if !matches!(needed, 5 | 15) || trait_level < 0 || !(0..=needed * 5).contains(&amount) { return None; }
    let whole = trait_level.checked_add(amount / needed)?;
    let remainder = amount % needed;
    if remainder == 0 { Some(whole as f32) }
    else { Some(whole.checked_add(1)? as f32 + (remainder - needed) as f32 / needed as f32) }
}

pub(super) fn arthropod(id: &str) -> bool {
    matches!(id.strip_prefix("minecraft:").unwrap_or(id), "spider" | "cave_spider" | "silverfish" | "endermite" | "bee")
}

pub(super) fn bane_duration(level: f32, roll: f32) -> Option<i32> {
    if !level.is_finite() || level <= 0.0 || !roll.is_finite() || !(0.0..1.0).contains(&roll) { return None; }
    let ticks = 20.0 + roll * level * 10.0;
    (ticks.is_finite() && ticks < i32::MAX as f32).then_some(ticks as i32)
}

pub(super) fn supported_bane_definition(world: &World) -> bool {
    let Some(raw) = world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()
        .and_then(|raw| raw.as_ref().ok()) else { return false; };
    let Some(bytes) = raw.0.get("tconstruct:tinkering/modifiers/bane_of_sssss").and_then(|layers| layers.last()) else { return false; };
    let target = serde_json::json!({"type":"mantle:or","predicates":[
        {"type":"mantle:mob_type","mobs":"minecraft:arthropod"},
        {"type":"mantle:tag","tag":"forge:creepers"}]});
    serde_json::from_slice::<serde_json::Value>(bytes).ok().is_some_and(|value|
        value == serde_json::json!({"level_display":"tconstruct:default","modules":[
            {"type":"tconstruct:conditional_melee_damage","each_level":2.0,"percent":false,"target":target.clone()},
            {"type":"tconstruct:weapon_mob_effect","effect":"minecraft:slowness",
             "hooks":["tconstruct:melee_hit","tconstruct:monster_melee_hit"],"level":{"flat":4.0},
             "target":target,"time":{"flat":20.0,"random":10.0}}
        ]}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_progress_merges_with_traits_in_source_order() {
        assert_eq!(effective_level(0,0), Some(0.0));
        assert_eq!(effective_level(0,5), Some(1.0));
        assert_eq!(effective_level(2,25), Some(7.0));
        assert_eq!(effective_level(2,1), Some(3.0 + -4.0 / 5.0));
        assert_eq!(effective_level(0,1), Some(1.0 + -4.0 / 5.0));
        assert_eq!(effective_level(-1,1), None);
        assert_eq!(effective_level(0,26), None);
        assert_eq!(effective_level(i32::MAX,1), None);
    }
    #[test]
    fn water_type_includes_amphibious_source_types_but_not_wet_undead() {
        for id in ["cod","salmon","pufferfish","tropical_fish","tadpole","dolphin","squid",
            "glow_squid","guardian","elder_guardian","axolotl","turtle"] {
            assert!(water_mob(id));
            assert!(water_mob(&format!("minecraft:{id}")));
            assert!(!water_mob(&format!("custom:{id}")));
        }
        for id in ["minecraft:drowned","minecraft:frog","minecraft:player","minecraft:zombie",
            "minecraft:creeper","minecraft:happy_ghast"] { assert!(!water_mob(id)); }
    }

    #[test]
    fn bane_has_fifteen_units_and_random_duration_scales_with_partial_level() {
        assert_eq!(effective_level_with_needed(0,15,15),Some(1.0));
        assert_eq!(effective_level_with_needed(2,1,15),Some(3.0 + -14.0 / 15.0));
        assert_eq!(effective_level_with_needed(0,76,15),None);
        assert_eq!(bane_duration(1.0,0.0),Some(20));
        assert_eq!(bane_duration(5.0,0.5),Some(45));
        assert_eq!(bane_duration(0.5,0.9),Some(24));
        assert_eq!(bane_duration(1.0,1.0),None);
        for id in ["spider","cave_spider","silverfish","endermite","bee"] { assert!(arthropod(id)); }
        assert!(!arthropod("creeper"));
        assert!(!arthropod("custom:spider"));
    }

}
