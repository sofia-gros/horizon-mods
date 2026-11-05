/// Source spells the volatile Elytra flag "elyta". This is an acquisition
/// contract, not a claim that the host consumes that TiC flag for flight.
pub(super) fn supported_definition(world: &super::tconstruct_world::World) -> bool {
    let Some(raw) = world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()
        .and_then(|raw| raw.as_ref().ok()) else { return false; };
    let Some(bytes) = raw.0.get("tconstruct:tinkering/modifiers/wings")
        .and_then(|layers| layers.last()) else { return false; };
    serde_json::from_slice::<serde_json::Value>(bytes).ok().is_some_and(|value|
        value == serde_json::json!({"level_display":"tconstruct:no_levels", "modules":[{
            "type":"tconstruct:volatile_flag", "flag":"tconstruct:elyta"}]}))
}
