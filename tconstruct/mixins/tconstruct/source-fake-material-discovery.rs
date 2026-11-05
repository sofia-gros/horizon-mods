//! Source acquisition goals mapped to a bounded, already validated world index.
use super::{Advancement, Tree};
use serde_json::{Value, json};

// Goal ID, source recipe ID, acquired item, generated direction prefix.
const FAMILIES: [(&str, &str, &str, &str); 2] = [
    ("tconstruct:recipes/misc/tools/parts/fake_ingot_to_block",
     "tconstruct:tools/parts/fake_ingot_to_block", "tconstruct:fake_ingot",
     "tconstruct:fake_metal_native/block/"),
    ("tconstruct:recipes/misc/tools/parts/fake_block_to_ingots",
     "tconstruct:tools/parts/fake_block_to_ingots", "tconstruct:fake_storage_block",
     "tconstruct:fake_metal_native/ingots/"),
];

fn source(family: usize) -> Value {
    let (_, recipe, item, _) = FAMILIES[family];
    json!({"parent":"minecraft:recipes/root", "criteria":{
        "has_item":{"trigger":"minecraft:inventory_changed", "conditions":{"items":[{"items":[item]}]}},
        "has_the_recipe":{"trigger":"minecraft:recipe_unlocked", "conditions":{"recipes":recipe}}
    }, "requirements":[["has_item","has_the_recipe"]], "rewards":{"recipes":[recipe]}})
}

fn valid_ids(family: usize, ids: &[String]) -> bool {
    ids.len() <= 512 && ids.windows(2).all(|pair| pair[0] < pair[1]) && ids.iter().all(|id| {
        id.strip_prefix(FAMILIES[family].3).is_some_and(|hex|
            !hex.is_empty() && hex.len() <= 512 && hex.len() % 2 == 0
                && hex.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
    })
}

fn mapped(family: usize, ids: &[String]) -> Value {
    let mut value = source(family);
    value["rewards"]["recipes"] = json!(ids);
    if ids.is_empty() {
        // Keep the original criterion names so earned source progress is not
        // erased, but do not complete an unavailable goal on inventory events.
        for name in ["has_item", "has_the_recipe"] {
            value["criteria"][name] = json!({"trigger":"minecraft:impossible", "conditions":{}});
        }
        return value;
    }
    let mut requirements = vec!["has_item".to_owned(), "has_the_recipe".to_owned()];
    for id in ids {
        let hex = id.strip_prefix(FAMILIES[family].3).expect("validated generated ID");
        let name = format!("has_native_recipe_{hex}");
        value["criteria"][name.as_str()] = json!({"trigger":"minecraft:recipe_unlocked", "conditions":{"recipes":id}});
        requirements.push(name);
    }
    value["requirements"] = json!([requirements]);
    value
}

// Match the complete host-supported shape, not just a goal name. Modified
// predicates, requirements, parents, displays or rewards are left untouched.
fn shape(a: &Advancement) -> Option<Value> {
    if a.display.is_some() || a.rewards.experience != 0 || !a.rewards.loot.is_empty() { return None; }
    let mut criteria = serde_json::Map::new();
    for c in &a.criteria {
        criteria.insert(c.name.clone(), json!({"trigger":c.trigger, "conditions":c.conditions.get()}));
    }
    Some(json!({"parent":a.parent, "criteria":criteria, "requirements":a.requirements,
        "rewards":{"recipes":a.rewards.recipes}}))
}

/// None means no change, preserving Arc identity and native inventory caches.
/// Existing list positions/IDs stay stable; all trigger indexes are rebuilt.
pub fn remap(tree: &Tree, variants: &[Vec<String>; 2]) -> Option<Tree> {
    let mut changes = Vec::new();
    for family in 0..2 {
        let Some(&at) = tree.by_id.get(FAMILIES[family].0) else { continue; };
        let old = &tree.list[at];
        let Some(old_shape) = shape(old) else { continue; };
        if old_shape != source(family)
            && !(valid_ids(family, &old.rewards.recipes) && old_shape == mapped(family, &old.rewards.recipes)) {
            continue;
        }
        let ids = if valid_ids(family, &variants[family]) { &variants[family][..] } else { &[] };
        let next = mapped(family, ids);
        if old_shape == next { continue; }
        if let Some(next) = super::parse(&old.id, &next) { changes.push((at, next)); }
    }
    if changes.is_empty() { return None; }
    let mut next = Tree { list: tree.list.clone(), by_id: tree.by_id.clone(), ..Tree::default() };
    for (at, changed) in changes { next.list[at] = changed; }
    for (ai, a) in next.list.iter().enumerate() {
        for (ci, c) in a.criteria.iter().enumerate() {
            next.by_trigger.entry(c.trigger.clone()).or_default().push((ai, ci));
        }
    }
    next.index_recipes();
    Some(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(family: usize, material: &str) -> String {
        format!("{}{}", FAMILIES[family].3,
            material.bytes().map(|b| format!("{b:02x}")).collect::<String>())
    }
    fn tree() -> Tree {
        let mut tree = Tree::default();
        for family in 0..2 {
            let a = super::super::parse(FAMILIES[family].0, &source(family)).unwrap();
            tree.by_id.insert(a.id.clone(), tree.list.len());
            tree.list.push(a);
        }
        tree
    }

    #[test]
    fn acquisition_or_any_variant_rewards_only_its_current_family() {
        let block = vec![id(0, "tconstruct:bronze"), id(0, "tconstruct:invar")];
        let ingots = vec![id(1, "tconstruct:bronze")];
        let mapped = remap(&tree(), &[block.clone(), ingots.clone()]).unwrap();
        assert_eq!(mapped.list[0].rewards.recipes, block);
        assert_eq!(mapped.list[1].rewards.recipes, ingots);
        assert_eq!(mapped.list[0].requirements.len(), 1);
        assert_eq!(mapped.list[0].requirements[0].len(), 4);
        assert!(mapped.list[0].criteria.iter().any(|c| c.name == "has_item"
            && c.conditions.get() == &source(0)["criteria"]["has_item"]["conditions"]));
        assert!(mapped.list[0].criteria.iter().any(|c| c.name == "has_the_recipe"));
        assert_eq!(mapped.recipe_criteria(&block[0]).len(), 1);
        assert_eq!(mapped.recipe_criteria(&ingots[0]).len(), 1);
        assert_eq!(mapped.recipe_criteria("other:recipe").len(), 0);
        assert!(remap(&mapped, &[block, ingots]).is_none());
    }

    #[test]
    fn missing_context_disables_then_reenables_without_renumbering_goals() {
        let original = tree();
        let off = remap(&original, &[vec![], vec![]]).unwrap();
        assert_eq!(off.by_id, original.by_id);
        assert!(off.list.iter().all(|a| a.rewards.recipes.is_empty()
            && a.criteria.iter().all(|c| c.trigger == "minecraft:impossible")));
        assert!(remap(&off, &[vec![], vec![]]).is_none());
        let block = id(0, "tconstruct:bronze");
        let on = remap(&off, &[vec![block.clone()], vec![]]).unwrap();
        assert_eq!(on.by_id, original.by_id);
        assert_eq!(on.list[0].rewards.recipes, vec![block]);
        assert!(on.by_trigger.contains_key("minecraft:inventory_changed"));
    }

    #[test]
    fn datapack_predicate_and_reward_changes_are_not_overwritten() {
        let mut tree = tree();
        tree.list[0].rewards.experience = 2;
        tree.list[1].requirements = vec![vec!["has_item".into()], vec!["has_the_recipe".into()]];
        assert!(remap(&tree, &[vec![id(0, "tconstruct:bronze")], vec![id(1, "tconstruct:bronze")]]).is_none());
    }

    #[test]
    fn malformed_or_other_family_candidates_disable_instead_of_unlocking() {
        let other = id(1, "tconstruct:bronze");
        let mapped = remap(&tree(), &[vec![other], vec![]]).unwrap();
        assert!(mapped.list[0].rewards.recipes.is_empty());
        let duplicate = id(0, "tconstruct:bronze");
        assert!(!valid_ids(0, &[duplicate.clone(), duplicate]));
        assert!(!valid_ids(0, &[format!("{}F0", FAMILIES[0].3)]));
    }
}
