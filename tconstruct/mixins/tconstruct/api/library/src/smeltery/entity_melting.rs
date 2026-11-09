//! Entity melting (`EntityMeltingRecipe` and `EntityMeltingModule`): a living thing inside a heating structure is hurt each second and gives
//! a little fluid for it, the fluid its kind is known for (honey from bees, glass from creepers), else liquid soul.
use serde_json::Value;

use super::alloy::{AlloyOutput, FluidMatch};
use crate::condition::{self, ConditionContext};
use crate::resource::ResourceId;

/// The damage a melted entity takes when its recipe does not say (`damage` defaults to 2).
pub const DEFAULT_DAMAGE: i32 = 2;
/// The fluid a melted entity gives when it has no recipe: liquid soul, a fifth of a glass pane (`FluidValues.GLASS_PANE` is 250).
pub const DEFAULT_FLUID: &str = "tconstruct:liquid_soul";
pub const DEFAULT_FLUID_AMOUNT: i32 = 250 / 5;

/// What an entity recipe applies to: one entity type or any of a tag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntityMatch {
    Type(ResourceId),
    /// Any of these types (`types`).
    Types(Vec<ResourceId>),
    Tag(ResourceId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityMeltingRecipe {
    pub id: ResourceId,
    /// Any one of these (the data writes one, a list of them, or a list of types).
    pub entity: Vec<EntityMatch>,
    pub result: AlloyOutput,
    /// The data the fluid carries (a potion's effect), as the data file writes it.
    pub result_data: Option<Value>,
    pub damage: i32,
}

/// Which entity types belong to an entity type tag.
pub trait EntityTags {
    fn types(&self, tag: &ResourceId) -> Vec<ResourceId>;
}

fn entity_matches(entity: &Value) -> Result<Vec<EntityMatch>, String> {
    if let Some(list) = entity.as_array() {
        return Ok(list.iter().map(entity_matches).collect::<Result<Vec<_>, _>>()?.into_iter().flatten().collect());
    }
    if let Some(kind) = entity.get("type").and_then(Value::as_str) {
        return Ok(vec![EntityMatch::Type(ResourceId::try_parse(kind).ok_or("bad entity type id")?)]);
    }
    if let Some(list) = entity.get("types").and_then(Value::as_array) {
        return Ok(vec![EntityMatch::Types(list.iter().map(|t| t.as_str().and_then(ResourceId::try_parse).ok_or("bad entity type id")).collect::<Result<Vec<_>, _>>()?)]);
    }
    let tag = entity.get("tag").and_then(Value::as_str).ok_or("the entity needs 'type', 'types' or 'tag'")?;
    Ok(vec![EntityMatch::Tag(ResourceId::try_parse(tag).ok_or("bad entity tag id")?)])
}

impl EntityMeltingRecipe {
    /// Reads a `tconstruct:entity_melting` file. `Ok(None)` when its data conditions are not met.
    pub fn from_json(id: ResourceId, json: &Value, conditions: &dyn ConditionContext) -> Result<Option<Self>, String> {
        if json.get("type").and_then(Value::as_str) != Some("tconstruct:entity_melting") {
            return Err("not an entity melting recipe".to_owned());
        }
        for c in json.get("conditions").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]) {
            if !condition::test(c, conditions)? {
                return Ok(None);
            }
        }
        let entity = entity_matches(json.get("entity").ok_or("an entity melting recipe needs an 'entity'")?)?;
        let result = json.get("result").ok_or("an entity melting recipe needs a 'result'")?;
        let fluid = if let Some(name) = result.get("fluid").and_then(Value::as_str) {
            FluidMatch::Fluid(ResourceId::try_parse(name).ok_or("bad fluid id")?)
        } else {
            FluidMatch::Tag(ResourceId::try_parse(result.get("tag").and_then(Value::as_str).ok_or("the result needs 'fluid' or 'tag'")?).ok_or("bad fluid tag id")?)
        };
        let amount = result.get("amount").and_then(Value::as_i64).filter(|a| *a >= 1).ok_or("the result needs a positive 'amount'")? as i32;
        let damage = json.get("damage").and_then(Value::as_i64).map_or(DEFAULT_DAMAGE, |d| d as i32);
        if damage < 1 {
            return Err("the damage must be at least 1".to_owned());
        }
        Ok(Some(Self { id, entity, result: AlloyOutput { fluid, amount }, result_data: result.get("nbt").cloned(), damage }))
    }

    pub fn matches(&self, entity_type: &ResourceId, tags: &dyn EntityTags) -> bool {
        self.entity.iter().any(|m| match m {
            EntityMatch::Type(kind) => kind == entity_type,
            EntityMatch::Types(list) => list.contains(entity_type),
            EntityMatch::Tag(tag) => tags.types(tag).contains(entity_type),
        })
    }
}

/// What the structure knows about an entity inside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entity {
    pub kind: ResourceId,
    pub alive: bool,
    /// On the melting blacklist tag.
    pub blacklisted: bool,
    /// Hurt by the fire the structure makes (not invulnerable to its damage), not a creative player, and not fire resistant.
    pub can_be_melted: bool,
}

/// What melting one entity does: the damage it takes and the fluid it gives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Melted {
    pub damage: i32,
    pub fluid: FluidMatch,
    pub amount: i32,
    pub data: Option<Value>,
}

/// `interactWithEntities` for one entity: None when it is not melted (dead, blacklisted, protected, or the structure cannot melt entities
/// now); else the damage and the fluid from its recipe, or the default ones. The entity must then take the damage for the fluid to be
/// added, which the caller decides.
pub fn melt(entity: &Entity, structure_can_melt: bool, recipes: &[EntityMeltingRecipe], tags: &dyn EntityTags) -> Option<Melted> {
    if !entity.alive || entity.blacklisted || !entity.can_be_melted || !structure_can_melt {
        return None;
    }
    Some(match recipes.iter().find(|r| r.matches(&entity.kind, tags)) {
        Some(recipe) => Melted { damage: recipe.damage, fluid: recipe.result.fluid.clone(), amount: recipe.result.amount, data: recipe.result_data.clone() },
        None => Melted { damage: DEFAULT_DAMAGE, fluid: FluidMatch::Fluid(ResourceId::try_parse(DEFAULT_FLUID).expect("a valid id")), amount: DEFAULT_FLUID_AMOUNT, data: None },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::condition::EmptyContext;
    use serde_json::json;

    fn id(text: &str) -> ResourceId {
        ResourceId::try_parse(text).unwrap()
    }

    struct Tags;
    impl EntityTags for Tags {
        fn types(&self, tag: &ResourceId) -> Vec<ResourceId> {
            if tag.to_string() == "forge:creepers" { vec![id("minecraft:creeper")] } else { Vec::new() }
        }
    }

    fn creeper() -> EntityMeltingRecipe {
        let json = json!({"type": "tconstruct:entity_melting", "damage": 3, "entity": {"tag": "forge:creepers"}, "result": {"amount": 50, "fluid": "tconstruct:molten_glass"}});
        EntityMeltingRecipe::from_json(id("tconstruct:creeper"), &json, &EmptyContext).unwrap().unwrap()
    }

    fn entity(kind: &str) -> Entity {
        Entity { kind: id(kind), alive: true, blacklisted: false, can_be_melted: true }
    }

    #[test]
    fn an_entity_with_a_recipe_takes_its_damage_and_gives_its_fluid() {
        let melted = melt(&entity("minecraft:creeper"), true, &[creeper()], &Tags).unwrap();
        assert_eq!((melted.damage, melted.amount), (3, 50));
        assert_eq!(melted.fluid, FluidMatch::Fluid(id("tconstruct:molten_glass")));
    }

    #[test]
    fn an_entity_without_a_recipe_gives_a_little_liquid_soul_for_two_damage() {
        let melted = melt(&entity("minecraft:cow"), true, &[creeper()], &Tags).unwrap();
        assert_eq!((melted.damage, melted.amount, melted.fluid), (2, 50, FluidMatch::Fluid(id("tconstruct:liquid_soul"))));
    }

    #[test]
    fn dead_blacklisted_or_protected_entities_and_a_structure_that_cannot_melt_give_nothing() {
        let recipes = [creeper()];
        let mut dead = entity("minecraft:creeper");
        dead.alive = false;
        assert_eq!(melt(&dead, true, &recipes, &Tags), None);
        let mut blacklisted = entity("minecraft:creeper");
        blacklisted.blacklisted = true;
        assert_eq!(melt(&blacklisted, true, &recipes, &Tags), None);
        let mut protected = entity("minecraft:creeper");
        protected.can_be_melted = false;
        assert_eq!(melt(&protected, true, &recipes, &Tags), None);
        assert_eq!(melt(&entity("minecraft:creeper"), false, &recipes, &Tags), None);
    }

    #[test]
    fn a_recipe_can_list_types_and_carry_data_on_its_fluid() {
        let json = json!({"type": "tconstruct:entity_melting", "entity": {"types": ["minecraft:bat", "minecraft:glow_squid"]},
            "result": {"amount": 50, "tag": "forge:potion", "nbt": {"Potion": "minecraft:night_vision"}}});
        let recipe = EntityMeltingRecipe::from_json(id("a:n"), &json, &EmptyContext).unwrap().unwrap();
        assert!(recipe.matches(&id("minecraft:bat"), &Tags) && recipe.matches(&id("minecraft:glow_squid"), &Tags) && !recipe.matches(&id("minecraft:cow"), &Tags));
        let melted = melt(&entity("minecraft:bat"), true, &[recipe], &Tags).unwrap();
        assert_eq!(melted.data, Some(json!({"Potion": "minecraft:night_vision"})));
    }

    #[test]
    fn a_list_of_entity_tags_matches_any_of_them() {
        let json = json!({"type": "tconstruct:entity_melting", "entity": [{"tag": "forge:creepers"}, {"type": "minecraft:axolotl"}], "result": {"amount": 50, "fluid": "a:b"}});
        let recipe = EntityMeltingRecipe::from_json(id("a:l"), &json, &EmptyContext).unwrap().unwrap();
        assert!(recipe.matches(&id("minecraft:creeper"), &Tags) && recipe.matches(&id("minecraft:axolotl"), &Tags) && !recipe.matches(&id("minecraft:cow"), &Tags));
    }

    #[test]
    fn a_recipe_for_one_type_matches_only_that_type_and_the_damage_defaults_to_two() {
        let json = json!({"type": "tconstruct:entity_melting", "entity": {"type": "minecraft:iron_golem"}, "result": {"amount": 90, "fluid": "tconstruct:molten_iron"}});
        let recipe = EntityMeltingRecipe::from_json(id("a:g"), &json, &EmptyContext).unwrap().unwrap();
        assert_eq!(recipe.damage, 2);
        assert!(recipe.matches(&id("minecraft:iron_golem"), &Tags) && !recipe.matches(&id("minecraft:snow_golem"), &Tags));
        assert!(EntityMeltingRecipe::from_json(id("a:b"), &json!({"type": "tconstruct:entity_melting", "entity": {"type": "a:b"}, "result": {"amount": 1, "fluid": "a:c"}, "damage": 0}), &EmptyContext).is_err());
    }
}
