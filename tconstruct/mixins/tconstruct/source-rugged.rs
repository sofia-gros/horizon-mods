const TERRAIN: &str = "tconstruct:rugged/terrain";
const ATTACKS: &str = "tconstruct:rugged/attacks";

pub(super) fn supported_value(value: &serde_json::Value) -> bool {
    value == &serde_json::json!({
        "level_display":"tconstruct:no_levels",
        "modules":[
            {"type":"tconstruct:block_damage","damage_source":{"type":"mantle:tag","tag":TERRAIN},"hooks":["tconstruct:damage_block"]},
            {"type":"tconstruct:block_damage","damage_source":{"type":"mantle:tag","tag":ATTACKS},"hooks":["tconstruct:damage_block"],"modifier_level":{"min":2}},
            {"type":"tconstruct:volatile_flag","flag":"tconstruct:snow_boots"}
        ]
    })
}

/// Any one intact piece must meet the condition. Two level-I pieces do not
/// satisfy a level-II modifier condition on either piece.
pub(super) fn blocked_types(
    levels: &[(i32,bool)],
    tags: &std::collections::HashMap<String,std::collections::HashSet<String>>,
) -> Option<Vec<String>> {
    let mut maximum = 0;
    for (level,broken) in levels {
        if *level < 0 { return None; }
        if !broken { maximum = maximum.max(*level); }
    }
    let mut blocked = std::collections::BTreeSet::new();
    for (tag,minimum) in [(TERRAIN,1),(ATTACKS,2)] {
        if maximum >= minimum {
            // An unreadable/missing tag disables only that module.
            if let Some(types) = tags.get(tag) { blocked.extend(types.iter().cloned()); }
        }
    }
    Some(blocked.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tags() -> std::collections::HashMap<String,std::collections::HashSet<String>> {
        [(TERRAIN.into(),["minecraft:cactus".into(),"minecraft:hot_floor".into()].into_iter().collect()),
         (ATTACKS.into(),["minecraft:sting".into(),"minecraft:thorns".into()].into_iter().collect())].into_iter().collect()
    }
    #[test]
    fn changed_conditions_are_not_assumed_to_be_the_stock_modifier() {
        let mut value = serde_json::json!({
            "level_display":"tconstruct:no_levels",
            "modules":[
                {"type":"tconstruct:block_damage","damage_source":{"type":"mantle:tag","tag":TERRAIN},"hooks":["tconstruct:damage_block"]},
                {"type":"tconstruct:block_damage","damage_source":{"type":"mantle:tag","tag":ATTACKS},"hooks":["tconstruct:damage_block"],"modifier_level":{"min":2}},
                {"type":"tconstruct:volatile_flag","flag":"tconstruct:snow_boots"}
            ]
        });
        assert!(supported_value(&value));
        value["modules"][1]["modifier_level"]["min"] = serde_json::json!(1);
        assert!(!supported_value(&value));
    }
    #[test]
    fn thresholds_are_per_piece_not_the_sum_of_levels() {
        let tags = tags();
        assert_eq!(blocked_types(&[(1,false),(1,false)],&tags).unwrap().len(),2);
        assert_eq!(blocked_types(&[(2,false)],&tags).unwrap().len(),4);
        assert!(blocked_types(&[(2,false)],&tags).unwrap().contains(&"minecraft:sting".into()));
    }
    #[test]
    fn broken_and_invalid_pieces_do_not_grant_immunity() {
        assert!(blocked_types(&[(2,true)],&tags()).unwrap().is_empty());
        assert_eq!(blocked_types(&[(-1,false)],&tags()),None);
    }
    #[test]
    fn missing_tag_disables_only_its_module() {
        let mut tags = tags();
        tags.remove(TERRAIN);
        assert_eq!(blocked_types(&[(2,false)],&tags).unwrap().len(),2);
    }
}
