/// Acquisition contract only. The source attribute is recorded by the modifier
/// definition, but no host movement path consumes extra jumps in this slice.
pub(super) fn supported_definition(world: &super::tconstruct_world::World) -> bool {
    let Some(raw) = world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()
        .and_then(|raw| raw.as_ref().ok()) else { return false; };
    let Some(bytes) = raw.0.get("tconstruct:tinkering/modifiers/double_jump")
        .and_then(|layers| layers.last()) else { return false; };
    serde_json::from_slice::<serde_json::Value>(bytes).ok().is_some_and(|value|
        value == serde_json::json!({"level_display":{"type":"tconstruct:unique",
            "first_unique":false,"unique_until":4},"modules":[{
            "type":"tconstruct:attribute","attribute":"tconstruct:player.jump_count",
            "each_level":1.0,"operation":"addition","slots":["feet","legs","chest","head"],
            "tooltip_style":"none"}]}))
}
