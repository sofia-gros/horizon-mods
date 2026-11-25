// The demo alone exercises Horizon's native item limit above vanilla's 99.
fn demo_stack_limit(item: &str, components: &mut serde_json::Map<String, serde_json::Value>) {
    if item == "pluto_wand:wand" && std::env::var_os("PLUTO_DEMO_STACK_LIMIT").is_some() {
        components.insert("minecraft:max_stack_size".into(), 200.into());
    }
}
