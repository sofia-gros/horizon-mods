// Current Tanned ranges and the reachable modifier-tag slice, not a guessed
// complete modifier registry. Reference: TConstruct f4e77d43, MIT.
use super::tconstruct_modifier_snapshot::Raw;
use std::collections::{BTreeMap, HashSet};
use horizon_items::items::ItemStack;

const KNOWN: &str = r###"{"tconstruct:protection":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:protection","damage_source":"mantle:can_protect","each_level":1.25}]},"tconstruct:thorns":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:thorns","chance":{"each_level":0.15},"constant_damage":{"flat":1},"damage_type":"minecraft:thorns","durability_usage":1,"random_damage":{"flat":3}}]},"tconstruct:fiery":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:fiery_attack","seconds":{"each_level":5}},{"type":"tconstruct:fiery_counter","chance":{"each_level":0.15},"constant_seconds":{"flat":1},"durability_usage":1,"random_seconds":{"flat":6},"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"}}]},"tconstruct:freezing":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:freezing_attack","seconds":{"each_level":4,"flat":4}},{"type":"tconstruct:freezing_counter","chance":{"each_level":0.15},"constant_seconds":{"flat":2},"durability_usage":1,"random_seconds":{"flat":6},"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"}}]},"tconstruct:springy":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:knockback","each_level":0.25},{"type":"tconstruct:knockback_counter","chance":{"each_level":0.15},"durability_usage":1,"flat_knockback":{"flat":0.5},"random_knockback":{"flat":0.5}}]},"tconstruct:pierce":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:stat_boost","each_level":0.5,"operation":"add","stat":"tconstruct:attack_damage"},{"type":"tconstruct:weapon_mob_effect","apply_before_melee":true,"effect":"tconstruct:pierce","level":{"per_level":1},"time":{"flat":80}}]},"tconstruct:venom":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:weapon_mob_effect","effect":"tconstruct:venom","level":{"flat":1},"time":{"flat":100,"random":100}},{"type":"tconstruct:counter_mob_effect","chance":{"flat":0.15},"effect":"tconstruct:venom","level":{"flat":1},"time":{"flat":100,"random":100}}],"priority":150},"tconstruct:conductive":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:weapon_mob_effect","effect":"tconstruct:conductive","level":{"flat":1},"time":{"flat":100,"random":100}},{"type":"tconstruct:counter_mob_effect","chance":{"flat":0.15},"effect":"tconstruct:conductive","level":{"flat":1},"time":{"flat":100,"random":100}}],"priority":150},"tconstruct:shock":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:conditional_melee_damage","formula":["$water","$level","*","$multiplier","*","$damage","+"],"percent":false,"variables":{"water":{"type":"tconstruct:entity","entity_type":"tconstruct:water","fallback":2,"which":"target"}}},{"type":"tconstruct:conditional_power","formula":["$water",2,"/","$level","*","$multiplier","*","$damage","+"],"percent":false,"variables":{"water":{"type":"tconstruct:entity","entity_type":"tconstruct:water","fallback":2,"which":"target"}}},{"type":"tconstruct:thorns","attacker":{"type":"mantle:or","predicates":["mantle:eyes_in_water","mantle:feet_in_water"]},"chance":{"each_level":0.2},"constant_damage":{"flat":2},"damage_type":"tconstruct:shock","durability_usage":1,"random_damage":{"flat":3},"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"}},{"type":"tconstruct:thorns","attacker":"mantle:raining_at","chance":{"each_level":0.1},"constant_damage":{"flat":2},"damage_type":"tconstruct:shock","durability_usage":1,"random_damage":{"flat":3},"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"}}]},"tconstruct:necrotic":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:lifesteal","durability_usage":{"each_level":1},"percentage":{"each_level":0.05}}]},"tconstruct:restore":{"level_display":"tconstruct:single_level","modules":[{"type":"tconstruct:restore_lost_health","chance":{"flat":0.15},"durability_usage":{"each_level":1},"effect_level":{"flat":1},"hooks":["tconstruct:modify_hurt","tconstruct:tooltip"],"percentage":{"flat":0.25},"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"}}]},"tconstruct:glowing":{"level_display":"tconstruct:single_level","modules":[{"type":"tconstruct:place_glow","tool_damage":5},{"type":"tconstruct:glow_walker","min_light":3,"radius":{"each_level":1.0,"flat":2.0},"tool_damage":5},{"type":"tconstruct:projectile_place_glow","blocks":true,"entities":true,"tool_damage":5},{"type":"tconstruct:show_offhand","mode":"disallow_broken"},{"type":"tconstruct:show_interaction_source"},{"type":"tconstruct:block_item_provider","item":"tconstruct:glow","tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/held"},"tool_damage":5}]}}"###;

fn tanned_definition() -> serde_json::Value {
    serde_json::json!({"level_display":"tconstruct:no_levels","modules":[
        {"type":"tconstruct:tool_damage_range","cause":{"type":"mantle:inverted",
            "inverted_type":"tconstruct:tag","tag":"tconstruct:durability/bypass_tanned"},"max":1},
        {"type":"tconstruct:tool_damage_range","cause":{"type":"tconstruct:tag",
            "tag":"tconstruct:durability/secondary"},"max":0}
    ],"priority":200})
}

fn definition(raw: &Raw, id: &str) -> Option<serde_json::Value> {
    let (namespace,path) = id.split_once(':')?;
    serde_json::from_slice(raw.0.get(&format!("{namespace}:tinkering/modifiers/{path}"))?.last()?).ok()
}

fn registered(raw: &Raw, id: &str) -> bool {
    // Source static registrations take precedence over dynamic files.
    if matches!(id,"tconstruct:empty" | "tconstruct:enderporting") { return true; }
    let Some(value) = definition(raw,id) else { return false; };
    if id == "tconstruct:tanned" && value == tanned_definition() { return true; }
    static VALUES: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    let known = VALUES.get_or_init(||serde_json::from_str(KNOWN).expect("retained source definitions"));
    known.get(id).is_some_and(|expected|expected == &value)
        || super::tconstruct_modifier_snapshot::known_moduleless_modifier(&value) == Some(true)
}

fn canonical(id: &str) -> String {
    if id.contains(':') { id.to_owned() } else { format!("minecraft:{id}") }
}

pub(super) fn member(raw: &Raw, tag: &str, cause: &str) -> Option<bool> {
    let tags: BTreeMap<String,Vec<Vec<u8>>> = raw.1.iter().filter_map(|(key,layers)| {
        let (namespace,path) = key.split_once(':')?;
        Some((format!("{namespace}:{}",path.strip_prefix("tinkering/tags/modifiers/")?),layers.clone()))
    }).collect();
    // An absent root is an empty tag in the complete resource snapshot.
    if !tags.contains_key(tag) { return Some(false); }
    fn visit(raw: &Raw, tags: &BTreeMap<String,Vec<Vec<u8>>>, tag: &str,
        visiting: &mut Vec<String>, registry: &mut HashSet<String>) -> Option<()> {
        if visiting.len() >= 64 || visiting.iter().any(|id|id == tag) { return None; }
        let files = tags.get(tag)?;
        if !files.iter().any(|file|super::tconstruct_source_tags::parse_file(file).is_some()) { return None; }
        visiting.push(tag.to_owned());
        for entry in super::tconstruct_source_tags::merge_tag_files(files) {
            if let Some(nested) = entry.id.strip_prefix('#') {
                let nested = canonical(nested);
                if !tags.contains_key(&nested) && (!entry.required || entry.remove) { continue; }
                visit(raw,tags,&nested,visiting,registry)?;
            } else {
                let id = canonical(&entry.id);
                // Unknown optional leaves are also unknown, not absent. Their
                // actual registration could change membership or removals.
                if !registered(raw,&id) { return None; }
                registry.insert(id);
            }
        }
        visiting.pop();
        Some(())
    }
    let mut registry = HashSet::new();
    visit(raw,&tags,tag,&mut Vec::new(),&mut registry)?;
    // Every reachable leaf was proved registered and canonical. No redirect
    // aliases are admitted, so ID removals equal modifier-object removals.
    super::tconstruct_source_tags::resolve_tag(tag,&tags,&registry).ok()
        .map(|ids|ids.iter().any(|id|id == cause))
}

pub(super) fn loss(raw: &Raw, amount: u32, level: i32, cause: Option<&str>) -> Option<u32> {
    if level <= 0 || amount == 0 { return Some(amount); }
    if definition(raw,"tconstruct:tanned")? != tanned_definition() { return None; }
    let cause = cause.unwrap_or("tconstruct:empty");
    let bypass = member(raw,"tconstruct:durability/bypass_tanned",cause)?;
    let secondary = member(raw,"tconstruct:durability/secondary",cause)?;
    Some(super::tconstruct_source_material_effects::tanned_wear(amount,level,bypass,secondary))
}

pub(super) fn wear(game: &super::Game<'_>, stack: &ItemStack, amount: u32, cause: Option<&str>) -> u32 {
    let projected = || -> Option<u32> {
        if stack.count != 1 || stack.wire_patch().is_some()
            || super::tconstruct_source_melee_shape::intact(stack).is_none() { return None; }
        let traits = super::tconstruct_source_material_traits::levels(&game.world,stack)?;
        let level = super::tconstruct_source_material_traits::level_with_upgrade(stack,&traits,"tconstruct:tanned")?;
        let raw = game.world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()?.as_ref().ok()?;
        loss(raw,amount,level,cause)
    };
    projected().unwrap_or(amount)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot() -> Raw {
        let mut raw = Raw::default();
        let known: BTreeMap<String,serde_json::Value> = serde_json::from_str(KNOWN).unwrap();
        for (id,value) in known {
            raw.0.insert(id.replace(':',":tinkering/modifiers/"),vec![serde_json::to_vec(&value).unwrap()]);
        }
        raw.0.insert("tconstruct:tinkering/modifiers/tanned".into(),vec![serde_json::to_vec(&tanned_definition()).unwrap()]);
        raw.1.insert("tconstruct:tinkering/tags/modifiers/durability/secondary".into(),
            vec![br#"{"values":["tconstruct:protection","tconstruct:thorns","tconstruct:enderporting"]}"#.to_vec()]);
        raw.1.insert("tconstruct:tinkering/tags/modifiers/durability/bypass_tanned".into(),
            vec![br##"{"values":["#tconstruct:durability/secondary"]}"##.to_vec()]);
        raw
    }
    #[test] fn source_ranges_and_empty_cause() {
        let raw = snapshot();
        assert_eq!(loss(&raw,7,1,None),Some(1));
        assert_eq!(loss(&raw,7,1,Some("tconstruct:protection")),Some(0));
        assert_eq!(loss(&raw,7,0,None),Some(7));
        assert_eq!(loss(&raw,0,1,None),Some(0));
    }
    #[test] fn live_replace_nested_remove_and_new_bypass() {
        let mut raw = snapshot();
        let secondary = "tconstruct:tinkering/tags/modifiers/durability/secondary";
        raw.1.get_mut(secondary).unwrap().push(br#"{"replace":true,"values":[]}"#.to_vec());
        assert_eq!(loss(&raw,7,1,Some("tconstruct:protection")),Some(1));
        raw.1.get_mut("tconstruct:tinkering/tags/modifiers/durability/bypass_tanned").unwrap()
            .push(br#"{"values":["tconstruct:empty"]}"#.to_vec());
        assert_eq!(loss(&raw,7,1,None),Some(7));
        raw.1.get_mut(secondary).unwrap().push(br#"{"values":["tconstruct:empty"]}"#.to_vec());
        assert_eq!(loss(&raw,7,1,None),Some(0));
        raw.1.get_mut(secondary).unwrap().push(br#"{"values":[],"remove":["tconstruct:empty"]}"#.to_vec());
        assert_eq!(loss(&raw,7,1,None),Some(7));
    }
    #[test] fn unknown_definitions_aliases_optional_entries_and_cycles_fail_closed() {
        let mut raw = snapshot();
        raw.0.insert("tconstruct:tinkering/modifiers/protection".into(),
            vec![br#"{"redirects":[{"id":"tconstruct:thorns"}]}"#.to_vec()]);
        assert_eq!(loss(&raw,7,1,None),None);
        let mut raw = snapshot();
        raw.1.get_mut("tconstruct:tinkering/tags/modifiers/durability/secondary").unwrap()
            .push(br#"{"values":[{"id":"example:unknown","required":false}]}"#.to_vec());
        assert_eq!(loss(&raw,7,1,None),None);
        let mut raw = snapshot();
        raw.1.get_mut("tconstruct:tinkering/tags/modifiers/durability/secondary").unwrap()
            .push(br##"{"values":["#tconstruct:durability/bypass_tanned"]}"##.to_vec());
        assert_eq!(loss(&raw,7,1,None),None);
        let mut raw = snapshot();
        raw.0.get_mut("tconstruct:tinkering/modifiers/tanned").unwrap()
            .push(br#"{"level_display":"tconstruct:no_levels"}"#.to_vec());
        assert_eq!(loss(&raw,7,1,None),None);
    }
}

#[cfg(test)] mod reinforced_tag_tests {
    use super::*;
    #[test] fn live_reinforced_bypass_uses_empty_cause_and_registered_glowing() {
        let mut raw = Raw::default();
        let values: BTreeMap<String, serde_json::Value> = serde_json::from_str(KNOWN).unwrap();
        for (id, value) in values {
            raw.0.insert(id.replace(':', ":tinkering/modifiers/"), vec![serde_json::to_vec(&value).unwrap()]);
        }
        let key = "tconstruct:tinkering/tags/modifiers/durability/bypass_reinforced";
        let tag = "tconstruct:durability/bypass_reinforced";
        raw.1.insert(key.into(), vec![br#"{"values":["tconstruct:glowing"]}"#.to_vec()]);
        assert_eq!(member(&raw, tag, "tconstruct:empty"), Some(false));
        raw.1.get_mut(key).unwrap().push(br#"{"replace":true,"values":["tconstruct:empty"]}"#.to_vec());
        assert_eq!(member(&raw, tag, "tconstruct:empty"), Some(true));
        raw.1.get_mut(key).unwrap().push(br#"{"values":["unknown:cause"]}"#.to_vec());
        assert_eq!(member(&raw, tag, "tconstruct:empty"), None);
    }
}
