// Intercept only the named shear capability; retain the original loot context
// for all other conditions and leave unrelated items untouched.
fn tconstruct_shear_loot(held:Option<&ItemStack>, condition:&serde_json::Value)->std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break,Continue};
    let name=condition.as_str().or_else(||condition.get("type").and_then(serde_json::Value::as_str));
    if name!=Some("minecraft:tool/can_shear") {return Continue(());}
    let Some(stack)=held else {return Continue(());};
    match self::tconstruct_source_shear::capability(stack) {Some(value)=>Break(value),None=>Continue(())}
}
