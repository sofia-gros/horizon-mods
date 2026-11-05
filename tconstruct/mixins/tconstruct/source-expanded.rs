// Shared by game and client_lib: no game-only sibling dependencies.
use horizon_items::items::ItemStack;
use serde_json::{Value,json};
pub(super) fn known(modifier:&[u8],definition:&[u8])->bool {
    if serde_json::from_slice::<Value>(modifier).ok()!=Some(json!({"level_display":"tconstruct:default","modules":[{"type":"tconstruct:volatile_int","each_level":1,"flag":"tconstruct:expanded"}]})) {return false;}
    let Ok(definition)=serde_json::from_slice::<Value>(definition) else {return false;};
    let Some(modules)=definition.get("modules").and_then(Value::as_array) else {return false;};
    let aoe:Vec<_>=modules.iter().filter(|m|m.get("type").and_then(Value::as_str).is_some_and(|s|s.ends_with("_aoe"))).collect();
    aoe==vec![&json!({"type":"tconstruct:box_aoe","bonus":{"height":1,"width":1},"expansion_direction":"tconstruct:side_hit","expansions":[{"width":1},{"height":1}]})]
}
pub(super) fn level(world:&super::tconstruct_world::World,stack:&ItemStack)->Option<i32> {
    if stack.item.as_ref()!="tconstruct:sledge_hammer" {return None;}
    let tags=world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    if !tags.get("tconstruct:modifiable/aoe")?.contains(stack.item.as_ref()) {return None;}
    let traits=super::tconstruct_source_material_traits::levels(world,stack)?;
    // Fireprimer also contributes this volatile flag; its unsupported route
    // must not be silently mistaken for a plain Expanded hammer.
    if super::tconstruct_source_material_traits::level_with_upgrade(stack,&traits,"tconstruct:fireprimer")?!=0 {return None;}
    let value=super::tconstruct_source_upgrade_rules::expanded_saved(stack)?.checked_add(traits.levels.get("tconstruct:expanded").copied().unwrap_or(0))?;
    if !(0..=8).contains(&value) {return None;}
    let raw=world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()?.as_ref().ok()?;
    let modifier=raw.0.get("tconstruct:tinkering/modifiers/expanded")?.last()?;
    let definition=world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?.get("tconstruct:tinkering/tool_definitions/sledge_hammer")?.last()?;
    known(modifier,definition).then_some(value)
}
