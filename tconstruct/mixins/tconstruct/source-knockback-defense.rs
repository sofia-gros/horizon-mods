use super::tconstruct_world::World;

pub(super) fn supported_definition(world: &World) -> bool {
    let Some(raw) = world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref().and_then(|raw| raw.as_ref().ok()) else { return false; };
    let Some(bytes) = raw.0.get("tconstruct:tinkering/modifiers/knockback_resistance")
        .and_then(|layers| layers.last()) else { return false; };
    serde_json::from_slice::<serde_json::Value>(bytes).ok().is_some_and(|value|
        value == serde_json::json!({"level_display":"tconstruct:single_level", "modules":[
            {"type":"tconstruct:stat_boost", "each_level":0.1, "operation":"add", "stat":"tconstruct:knockback_resistance"},
            {"type":"tconstruct:material_variant_color", "material":"tconstruct:wool"}
        ]}))
}
