use super::tconstruct_world::World;

pub(super) fn supported_definition(world: &World) -> bool {
    let Some(raw) = world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()
        .and_then(|raw| raw.as_ref().ok()) else { return false; };
    let Some(bytes) = raw.0.get("tconstruct:tinkering/modifiers/smite").and_then(|layers| layers.last()) else { return false; };
    serde_json::from_slice::<serde_json::Value>(bytes).ok().is_some_and(|value|
        value == serde_json::json!({"level_display":"tconstruct:default", "modules":[{
            "type":"tconstruct:conditional_melee_damage", "each_level":2.0, "percent":false,
            "target":{"type":"mantle:mob_type", "mobs":"minecraft:undead"}
        }]}))
}

pub(super) fn effective_level(trait_level: i32, amount: i32) -> Option<f32> {
    if trait_level < 0 || !(0..=25).contains(&amount) { return None; }
    let whole = trait_level.checked_add(amount / 5)?;
    let remainder = amount % 5;
    if remainder == 0 { Some(whole as f32) }
    else { Some(whole.checked_add(1)? as f32 + (remainder - 5) as f32 / 5.0) }
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
}
