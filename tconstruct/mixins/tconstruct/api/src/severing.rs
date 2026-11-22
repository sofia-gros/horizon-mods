//! Severing recipes (`tconstruct:severing`) and the roll of SeveringModule, from SlimeKnights SeveringRecipe and SeveringModule (MIT; see
//! LICENSE): a mob a severing tool kills may drop the recipe's item (a head, a rod, a cobweb), with a chance of the modifier level times the
//! recipe's base chance plus its looting bonus times the looting level. The mob is told by its entity type; the recipes whose output depends on
//! the entity's own state (a baby, a sheared sheep, a player's skin) are not loaded, as the loot event reports no such state.
use pluto_lib::{Identifier, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::Value;
use std::sync::{Arc, RwLock};

pub(crate) const FOLDER: &str = "recipes/tools/severing";
const SEVERING: &str = "tconstruct:severing";
/// The recipe types that read the entity's own state: not loaded, and not a problem.
const STATEFUL: &[&str] = &[
    "tconstruct:ageable_severing",
    "tconstruct:mooshroom_demushrooming",
    "tconstruct:player_beheading",
    "tconstruct:sheep_shearing",
    "tconstruct:snow_golem_beheading",
];
/// What Forge's `forge:heads` item tag holds in the game itself (the vanilla heads) and what the original adds to it
/// (data/forge/tags/items/heads.json); a mob that already drops one gets no more.
const HEADS: &[&str] = &[
    "minecraft:creeper_head",
    "minecraft:dragon_head",
    "minecraft:piglin_head",
    "minecraft:player_head",
    "minecraft:skeleton_skull",
    "minecraft:wither_skeleton_skull",
    "minecraft:zombie_head",
    "tconstruct:blaze_head",
    "tconstruct:enderman_head",
    "tconstruct:stray_head",
    "tconstruct:husk_head",
    "tconstruct:drowned_head",
    "tconstruct:spider_head",
    "tconstruct:cave_spider_head",
    "tconstruct:piglin_brute_head",
    "tconstruct:zombified_piglin_head",
];

/// Whether `item` is one of the heads.
pub(crate) fn is_head(item: &str) -> bool {
    HEADS.contains(&item)
}

/// EntityIngredient: one entity type, a list of them, or a tag.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Entities {
    Types(Vec<Identifier>),
    Tag(Identifier),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Recipe {
    pub id: Identifier,
    pub entities: Entities,
    pub item: Identifier,
    pub count: u32,
    pub base_chance: f32,
    pub looting_bonus: f32,
}

fn percent(
    fields: &serde_json::Map<String, Value>,
    key: &str,
    default: f32,
) -> Result<f32, String> {
    match fields.get(key) {
        None => Ok(default),
        Some(Value::Number(number)) => {
            let value = number
                .as_f64()
                .ok_or_else(|| format!("{key} must be a number"))? as f32;
            if (0.0..=1.0).contains(&value) {
                Ok(value)
            } else {
                Err(format!("{key} must be between 0 and 1"))
            }
        }
        Some(_) => Err(format!("{key} must be a number")),
    }
}

impl Recipe {
    fn decode(id: &Identifier, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("recipe must be an object")?;
        let entity = fields
            .get("entity")
            .and_then(Value::as_object)
            .ok_or("entity must be an object")?;
        let identifier = |value: &Value| {
            value
                .as_str()
                .ok_or_else(|| "entity must be an ID string".to_owned())
                .and_then(|text| Identifier::parse(text).map_err(|error| error.to_string()))
        };
        let entities = if let Some(kind) = entity.get("type") {
            Entities::Types(vec![identifier(kind)?])
        } else if let Some(kinds) = entity.get("types") {
            Entities::Types(
                kinds
                    .as_array()
                    .ok_or("types must be an array")?
                    .iter()
                    .map(identifier)
                    .collect::<Result<_, _>>()?,
            )
        } else if let Some(tag) = entity.get("tag") {
            Entities::Tag(identifier(tag)?)
        } else {
            return Err("entity must name a type, types or a tag".into());
        };
        // ItemOutput.Loadable.REQUIRED_STACK: an item ID, or an object with the item and a count.
        let (item, count) = match fields.get("result") {
            Some(Value::String(text)) => (
                Identifier::parse(text).map_err(|error| error.to_string())?,
                1,
            ),
            Some(Value::Object(result)) => {
                let item = Identifier::parse(
                    result
                        .get("item")
                        .and_then(Value::as_str)
                        .ok_or("result item must be an ID string")?,
                )
                .map_err(|error| error.to_string())?;
                let count = match result.get("count") {
                    None => 1,
                    Some(count) => count
                        .as_u64()
                        .filter(|count| (1..=99).contains(count))
                        .ok_or("result count must be from 1 to 99")?
                        as u32,
                };
                (item, count)
            }
            _ => return Err("result must be an item".into()),
        };
        Ok(Self {
            id: id.clone(),
            entities,
            item,
            count,
            base_chance: percent(fields, "per_level_chance", 0.05)?,
            looting_bonus: percent(fields, "looting_bonus", 0.01)?,
        })
    }

    /// SeveringRecipe.matches: the recipe's entity ingredient accepts the entity type.
    pub fn matches(&self, entity: &Identifier, in_tag: &dyn Fn(&Identifier) -> bool) -> bool {
        match &self.entities {
            Entities::Types(kinds) => kinds.contains(entity),
            Entities::Tag(tag) => in_tag(tag),
        }
    }

    /// SeveringRecipe.getChance.
    pub fn chance(&self, level: f32, looting: f32) -> f32 {
        level * (self.base_chance + self.looting_bonus * looting)
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Recipes {
    recipes: Vec<Recipe>,
    pub problems: Vec<String>,
    pub loaded: bool,
}

impl Recipes {
    pub fn load(files: &[(Identifier, Value)]) -> Self {
        let mut result = Self {
            loaded: true,
            ..Self::default()
        };
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            match value.get("type").and_then(Value::as_str) {
                Some(SEVERING) => match Recipe::decode(id, value) {
                    Ok(recipe) => result.recipes.push(recipe),
                    Err(error) => result.problems.push(format!("{id}: {error}")),
                },
                Some(kind) if STATEFUL.contains(&kind) => {}
                _ => {}
            }
        }
        result
    }
    pub fn len(&self) -> usize {
        self.recipes.len()
    }

    /// SeveringRecipeCache.findRecipe: the recipes that accept the entity type, in recipe ID order.
    pub fn find<'a>(
        &'a self,
        entity: &'a Identifier,
        in_tag: &'a dyn Fn(&Identifier) -> bool,
    ) -> impl Iterator<Item = &'a Recipe> + 'a {
        self.recipes
            .iter()
            .filter(move |recipe| recipe.matches(entity, in_tag))
    }

    /// SeveringModule.processLoot for one severing module of effective `level`: nothing when the loot already holds a head; otherwise each
    /// recipe for the mob rolls its chance (doubled for a rare mob) and adds its item, a count above one being a random count from one to it.
    /// `random` gives values in [0, 1).
    pub fn process(
        &self,
        entity: &Identifier,
        in_tag: &dyn Fn(&Identifier) -> bool,
        rare: bool,
        level: f32,
        looting: f32,
        loot: &mut Vec<pluto_lib::ItemSlot>,
        random: &mut dyn FnMut() -> f32,
    ) {
        if loot
            .iter()
            .any(|slot| slot.count > 0 && is_head(&slot.item.to_string()))
        {
            return;
        }
        let multiplier = if rare { 2.0 } else { 1.0 };
        let wins: Vec<&Recipe> = self.find(entity, in_tag).collect();
        for recipe in wins {
            if random() < recipe.chance(level, looting) * multiplier {
                let count = if recipe.count > 1 {
                    ((random() * recipe.count as f32) as u32).min(recipe.count - 1) + 1
                } else {
                    recipe.count
                };
                loot.push(pluto_lib::ItemSlot::new(recipe.item.clone(), count));
            }
        }
    }
}

/// Reload listener, shaped like the other recipe listeners.
pub(crate) fn register() -> Result<Arc<RwLock<Recipes>>, String> {
    let state = Arc::new(RwLock::new(Recipes::default()));
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "severing_recipes").unwrap(),
        FOLDER,
        move |files| {
            let mut state = output.write().map_err(|_| "severing recipes poisoned")?;
            *state = Recipes::load(files);
            for problem in &state.problems {
                eprintln!("Tinkers: {problem}");
            }
            Ok(())
        },
    )?;
    let stopped = state.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| {
        *stopped.write().expect("severing recipes poisoned") = Recipes::default()
    });
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn id(text: &str) -> Identifier {
        Identifier::parse(text).unwrap()
    }
    fn recipes() -> Recipes {
        Recipes::load(&[
            (
                id("t:head"),
                json!({"type": SEVERING, "entity": {"type": "minecraft:creeper"}, "result": "minecraft:creeper_head"}),
            ),
            (
                id("t:rod"),
                json!({"type": SEVERING, "entity": {"type": "minecraft:blaze"}, "result": {"item": "minecraft:blaze_rod", "count": 2}, "per_level_chance": 0.5, "looting_bonus": 0.1}),
            ),
            (
                id("t:web"),
                json!({"type": SEVERING, "entity": {"types": ["minecraft:spider", "minecraft:cave_spider"]}, "result": "minecraft:cobweb"}),
            ),
            (
                id("t:feather"),
                json!({"type": "tconstruct:ageable_severing", "entity": {"type": "minecraft:chicken"}, "adult_result": "minecraft:feather"}),
            ),
            (id("t:sheep"), json!({"type": "tconstruct:sheep_shearing"})),
        ])
    }
    fn none(_: &Identifier) -> bool {
        false
    }

    #[test]
    fn recipes_of_state_dependent_types_are_skipped_and_the_rest_decode() {
        let loaded = recipes();
        assert_eq!((loaded.len(), loaded.problems.len()), (3, 0));
        assert_eq!(loaded.find(&id("minecraft:cave_spider"), &none).count(), 1);
        assert_eq!(
            loaded.find(&id("minecraft:chicken"), &none).count(),
            0,
            "a baby's output is told by state the loot event lacks"
        );
        assert!(Recipes::load(&[(id("t:bad"), json!({"type": SEVERING, "entity": {"type": "minecraft:cow"}, "result": "minecraft:leather", "per_level_chance": 1.5}))]).problems[0].contains("between 0 and 1"));
    }

    #[test]
    fn chance_is_level_times_base_plus_looting_bonus() {
        let loaded = recipes();
        let (blaze, creeper) = (id("minecraft:blaze"), id("minecraft:creeper"));
        let rod = loaded.find(&blaze, &none).next().unwrap();
        assert!((rod.chance(2.0, 3.0) - 2.0 * (0.5 + 0.1 * 3.0)).abs() < 1e-6);
        let head = loaded.find(&creeper, &none).next().unwrap();
        assert!(
            (head.chance(1.0, 0.0) - 0.05).abs() < 1e-6,
            "the defaults are 5% a level and 1% a looting level"
        );
    }

    #[test]
    fn a_roll_adds_the_item_unless_the_loot_has_a_head_and_a_count_is_random_up_to_it() {
        let loaded = recipes();
        let mut loot = Vec::new();
        // Chance 1.0 at level 2, looting 3 (0.5 + 0.3 = 0.8, times 2 is above one), rolling 0 each time: the count is 1 + floor(0 * 2).
        loaded.process(
            &id("minecraft:blaze"),
            &none,
            false,
            2.0,
            3.0,
            &mut loot,
            &mut || 0.0,
        );
        assert_eq!(
            (loot.len(), loot[0].count, loot[0].item.to_string()),
            (1, 1, "minecraft:blaze_rod".to_owned())
        );
        let mut rolls = [0.0, 0.99].into_iter();
        let mut loot = Vec::new();
        loaded.process(
            &id("minecraft:blaze"),
            &none,
            false,
            2.0,
            3.0,
            &mut loot,
            &mut || rolls.next().unwrap(),
        );
        assert_eq!(loot[0].count, 2, "the top of the range");
        // Nothing when the roll misses, and a rare mob doubles the chance.
        let mut loot = Vec::new();
        loaded.process(
            &id("minecraft:creeper"),
            &none,
            false,
            1.0,
            0.0,
            &mut loot,
            &mut || 0.07,
        );
        assert!(loot.is_empty());
        loaded.process(
            &id("minecraft:creeper"),
            &none,
            true,
            1.0,
            0.0,
            &mut loot,
            &mut || 0.07,
        );
        assert_eq!(loot.len(), 1);
        // A head already among the loot ends it.
        let mut with_head = vec![pluto_lib::ItemSlot::new(id("minecraft:skeleton_skull"), 1)];
        loaded.process(
            &id("minecraft:creeper"),
            &none,
            true,
            20.0,
            0.0,
            &mut with_head,
            &mut || 0.0,
        );
        assert_eq!(with_head.len(), 1);
    }
}
