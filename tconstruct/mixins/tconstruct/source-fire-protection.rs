/// Acquisition contract. The source's protection and secondary enchantment
/// effects are deliberately not replaced with vanilla enchantment protection.
pub(super) fn supported_definition(world: &super::tconstruct_world::World) -> bool {
    let Some(raw) = world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()
        .and_then(|raw| raw.as_ref().ok()) else { return false; };
    let Some(bytes)=raw.0.get("tconstruct:tinkering/modifiers/fire_protection")
        .and_then(|layers| layers.last()) else { return false; };
    serde_json::from_slice::<serde_json::Value>(bytes).ok().is_some_and(|value|
        value == serde_json::json!({"level_display":"tconstruct:default","modules":[
            {"type":"tconstruct:enchantment_ignoring_protection","level":1,"name":"minecraft:fire_protection"},
            {"type":"tconstruct:protection","damage_source":{"type":"mantle:and","predicates":[
                "mantle:can_protect",{"type":"mantle:tag","tag":"tconstruct:protection/fire"}]},"each_level":3.0}
        ]}))
}
