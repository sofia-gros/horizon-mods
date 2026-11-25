//! Severing recipes (`tconstruct:severing`) and the roll of SeveringModule, from SlimeKnights SeveringRecipe and SeveringModule (MIT; see
//! LICENSE): a mob a severing tool kills may drop the recipe's item (a head, a rod, a cobweb), with a chance of the modifier level times the
//! recipe's base chance plus its looting bonus times the looting level. The mob is told by its entity type; the recipes whose output depends on
//! the entity's own state (a baby, a sheared sheep, a player's skin) read that state from the loot event, which reports it with the drops.
use pluto_lib::{Identifier, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::Value;
use std::sync::{Arc, RwLock};

pub(crate) const FOLDER: &str = "recipes/tools/severing";
const SEVERING: &str = "tconstruct:severing";
const AGEABLE: &str = "tconstruct:ageable_severing";
const MOOSHROOM: &str = "tconstruct:mooshroom_demushrooming";
const PLAYER_BEHEADING: &str = "tconstruct:player_beheading";
const SHEEP_SHEARING: &str = "tconstruct:sheep_shearing";
const SNOW_GOLEM: &str = "tconstruct:snow_golem_beheading";
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
/// DyeColor's wool in dye order, white first (SheepShearingRecipe.WOOL_BY_COLOR).
const WOOL: [&str; 16] = [
    "white", "orange", "magenta", "light_blue", "yellow", "lime", "pink", "gray", "light_gray", "cyan", "purple", "blue",
    "brown", "green", "red", "black",
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

/// What a winning recipe drops (SeveringRecipe.getOutput, which the state-dependent kinds override to read the mob itself).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Output {
    /// The same item for every mob (SeveringRecipe).
    Item { item: Identifier, count: u32 },
    /// AgeableSeveringRecipe: a baby drops the child result (nothing when the recipe names none), an adult the adult result.
    Ageable { adult: (Identifier, u32), child: Option<(Identifier, u32)> },
    /// MooshroomDemushroomingRecipe: an adult drops five mushrooms of its own kind; a baby drops nothing.
    Mooshroom,
    /// PlayerBeheadingRecipe: the player's head, keeping their profile; a plain head when no profile was reported.
    PlayerHead,
    /// SheepShearingRecipe: a grown sheep that still has its wool drops two wool of its colour; a sheared or baby sheep drops nothing.
    SheepWool,
    /// SnowGolemBeheadingRecipe: a golem still wearing its pumpkin drops the carved pumpkin; one without drops a snow block.
    SnowGolem,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Recipe {
    pub id: Identifier,
    pub entities: Entities,
    pub output: Output,
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

/// EntityIngredient: a `type`, a `types` list or a `tag` in the recipe's `entity` object.
fn entity_ingredient(fields: &serde_json::Map<String, Value>) -> Result<Entities, String> {
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
    if let Some(kind) = entity.get("type") {
        Ok(Entities::Types(vec![identifier(kind)?]))
    } else if let Some(kinds) = entity.get("types") {
        Ok(Entities::Types(
            kinds
                .as_array()
                .ok_or("types must be an array")?
                .iter()
                .map(identifier)
                .collect::<Result<_, _>>()?,
        ))
    } else if let Some(tag) = entity.get("tag") {
        Ok(Entities::Tag(identifier(tag)?))
    } else {
        Err("entity must name a type, types or a tag".into())
    }
}

/// ItemOutput.Loadable: an item ID, or an object with the item and a count from 1 to 99.
fn item_output(value: Option<&Value>, field: &str) -> Result<(Identifier, u32), String> {
    match value {
        Some(Value::String(text)) => Ok((
            Identifier::parse(text).map_err(|error| error.to_string())?,
            1,
        )),
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
            Ok((item, count))
        }
        _ => Err(format!("{field} must be an item")),
    }
}

/// A player's profile as a head keeps it (the original's SkullOwner, written from the game profile): the name and the id
/// as the uuid's four ints. The game reports no texture properties for the player, so the list is empty and the client
/// resolves the skin from the name and id, as it does for a profile without properties.
fn player_profile(profile: &pluto_lib::PlayerProfile) -> pluto_lib::NbtCompound {
    let uuid = profile.uuid;
    let mut compound = pluto_lib::NbtCompound::new();
    compound.insert("name".to_owned(), pluto_lib::Nbt::String(profile.name.clone()));
    compound.insert(
        "id".to_owned(),
        pluto_lib::Nbt::IntArray(vec![
            (uuid >> 96) as i32,
            (uuid >> 64) as i32,
            (uuid >> 32) as i32,
            uuid as i32,
        ]),
    );
    compound.insert("properties".to_owned(), pluto_lib::Nbt::List(Vec::new()));
    compound
}

impl Recipe {
    fn decode(id: &Identifier, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("recipe must be an object")?;
        let (item, count) = item_output(fields.get("result"), "result")?;
        Ok(Self {
            id: id.clone(),
            entities: entity_ingredient(fields)?,
            output: Output::Item { item, count },
            base_chance: percent(fields, "per_level_chance", 0.05)?,
            looting_bonus: percent(fields, "looting_bonus", 0.01)?,
        })
    }

    /// AgeableSeveringRecipe's loader: the adult result is required, the child result is not (Mantle's OPTIONAL_STACK,
    /// an empty stack when absent).
    fn decode_ageable(id: &Identifier, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("recipe must be an object")?;
        let adult = item_output(fields.get("adult_result"), "adult_result")?;
        let child = match fields.get("child_result") {
            None => None,
            Some(child) => Some(item_output(Some(child), "child_result")?),
        };
        Ok(Self {
            id: id.clone(),
            entities: entity_ingredient(fields)?,
            output: Output::Ageable { adult, child },
            base_chance: percent(fields, "per_level_chance", 0.05)?,
            looting_bonus: percent(fields, "looting_bonus", 0.01)?,
        })
    }

    /// The special kinds (built by SpecialSeveringRecipeBuilder): the entity and the output are the recipe class's own,
    /// so the data names only the chances.
    fn special(id: &Identifier, entity: &str, output: Output, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("recipe must be an object")?;
        Ok(Self {
            id: id.clone(),
            entities: Entities::Types(vec![
                Identifier::parse(entity).map_err(|error| error.to_string())?,
            ]),
            output,
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

    /// SeveringRecipe.getOutput for the mob whose state `facts` reports: the stack the recipe drops for it, or None for
    /// the original's empty stack (a baby with no child result, a sheared sheep, a baby mooshroom). The state-dependent
    /// kinds drop nothing when the loot event reported no state; a beheading whose profile was not reported drops the
    /// plain head the original drops for an entity that is not a player.
    pub fn output_for(&self, facts: Option<&pluto_lib::LootEntityFacts>) -> Option<pluto_lib::ItemSlot> {
        match &self.output {
            Output::Item { item, count } => Some(pluto_lib::ItemSlot::new(item.clone(), *count)),
            Output::Ageable { adult, child } => {
                let facts = facts?;
                let (item, count) = if facts.baby { child.as_ref()? } else { adult };
                Some(pluto_lib::ItemSlot::new(item.clone(), *count))
            }
            Output::Mooshroom => {
                let facts = facts?;
                if facts.baby {
                    return None;
                }
                let item = if facts.mooshroom.as_deref() == Some("brown") {
                    "minecraft:brown_mushroom"
                } else {
                    "minecraft:red_mushroom"
                };
                Some(pluto_lib::ItemSlot::new(Identifier::parse(item).ok()?, 5))
            }
            Output::PlayerHead => {
                let mut slot = pluto_lib::ItemSlot::new(Identifier::parse("minecraft:player_head").ok()?, 1);
                if let Some(profile) = facts.and_then(|facts| facts.player.as_ref()) {
                    slot.profile = Some(player_profile(profile));
                }
                Some(slot)
            }
            Output::SheepWool => {
                let facts = facts?;
                if facts.baby || facts.sheared {
                    return None;
                }
                let colour = WOOL.get(facts.wool_color.unwrap_or(0) as usize)?;
                Some(pluto_lib::ItemSlot::new(
                    Identifier::new("minecraft", &format!("{colour}_wool")).ok()?,
                    2,
                ))
            }
            Output::SnowGolem => {
                let facts = facts?;
                let item = if facts.sheared { "minecraft:snow_block" } else { "minecraft:carved_pumpkin" };
                Some(pluto_lib::ItemSlot::new(Identifier::parse(item).ok()?, 1))
            }
        }
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
            let decoded = match value.get("type").and_then(Value::as_str) {
                Some(SEVERING) => Some(Recipe::decode(id, value)),
                Some(AGEABLE) => Some(Recipe::decode_ageable(id, value)),
                Some(MOOSHROOM) => Some(Recipe::special(id, "minecraft:mooshroom", Output::Mooshroom, value)),
                Some(PLAYER_BEHEADING) => Some(Recipe::special(id, "minecraft:player", Output::PlayerHead, value)),
                Some(SHEEP_SHEARING) => Some(Recipe::special(id, "minecraft:sheep", Output::SheepWool, value)),
                Some(SNOW_GOLEM) => Some(Recipe::special(id, "minecraft:snow_golem", Output::SnowGolem, value)),
                _ => None,
            };
            match decoded {
                Some(Ok(recipe)) => result.recipes.push(recipe),
                Some(Err(error)) => result.problems.push(format!("{id}: {error}")),
                None => {}
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
    /// recipe for the mob rolls its chance (doubled for a rare mob) and adds its output for that mob (an empty output adds nothing), a count
    /// above one being a random count from one to it. `facts` is the mob's own state as the loot event reports it. `random` gives values in [0, 1).
    pub fn process(
        &self,
        entity: &Identifier,
        in_tag: &dyn Fn(&Identifier) -> bool,
        rare: bool,
        level: f32,
        looting: f32,
        facts: Option<&pluto_lib::LootEntityFacts>,
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
                let Some(mut slot) = recipe.output_for(facts) else {
                    continue;
                };
                if slot.count > 1 {
                    slot.count = ((random() * slot.count as f32) as u32).min(slot.count - 1) + 1;
                }
                loot.push(slot);
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
    /// The seven shipped state-dependent recipes, as the content ships them.
    fn shipped() -> Recipes {
        Recipes::load(&[
            (
                id("tconstruct:chicken_feather"),
                json!({"type": "tconstruct:ageable_severing", "adult_result": {"count": 2, "item": "minecraft:feather"}, "entity": {"type": "minecraft:chicken"}, "looting_bonus": 0.01, "per_level_chance": 0.05}),
            ),
            (
                id("tconstruct:rabbit_foot"),
                json!({"type": "tconstruct:ageable_severing", "adult_result": "minecraft:rabbit_foot", "entity": {"type": "minecraft:rabbit"}, "looting_bonus": 0.01, "per_level_chance": 0.05}),
            ),
            (
                id("tconstruct:turtle_shell"),
                json!({"type": "tconstruct:ageable_severing", "adult_result": "minecraft:turtle_helmet", "child_result": "minecraft:scute", "entity": {"type": "minecraft:turtle"}, "looting_bonus": 0.01, "per_level_chance": 0.05}),
            ),
            (
                id("tconstruct:mooshroom_shroom"),
                json!({"type": "tconstruct:mooshroom_demushrooming", "looting_bonus": 0.01, "per_level_chance": 0.05}),
            ),
            (
                id("tconstruct:player_head"),
                json!({"type": "tconstruct:player_beheading", "looting_bonus": 0.02, "per_level_chance": 0.1}),
            ),
            (
                id("tconstruct:sheep_wool"),
                json!({"type": "tconstruct:sheep_shearing", "looting_bonus": 0.01, "per_level_chance": 0.05}),
            ),
            (
                id("tconstruct:snow_golem_head"),
                json!({"type": "tconstruct:snow_golem_beheading", "looting_bonus": 0.01, "per_level_chance": 0.05}),
            ),
        ])
    }
    fn none(_: &Identifier) -> bool {
        false
    }
    fn facts() -> pluto_lib::LootEntityFacts {
        pluto_lib::LootEntityFacts::default()
    }
    /// Runs one mob's severing at a level that always wins (level 20 at the shipped 5% is a chance of 1), with the given
    /// state and rolls after the winning chance roll, and returns what dropped.
    fn drops(
        loaded: &Recipes,
        entity: &str,
        state: Option<&pluto_lib::LootEntityFacts>,
        rolls: &[f32],
    ) -> Vec<pluto_lib::ItemSlot> {
        let mut sequence = std::iter::once(0.0).chain(rolls.iter().copied());
        let mut loot = Vec::new();
        loaded.process(&id(entity), &none, false, 20.0, 0.0, state, &mut loot, &mut || {
            sequence.next().unwrap_or(0.0)
        });
        loot
    }

    #[test]
    fn every_recipe_kind_loads_and_the_rest_decode() {
        let loaded = recipes();
        assert_eq!((loaded.len(), loaded.problems.len()), (5, 0));
        assert_eq!(loaded.find(&id("minecraft:cave_spider"), &none).count(), 1);
        assert_eq!(loaded.find(&id("minecraft:chicken"), &none).count(), 1, "the ageable kind loads");
        assert_eq!(loaded.find(&id("minecraft:sheep"), &none).count(), 1, "the special kinds load with their own entity");
        assert!(Recipes::load(&[(id("t:bad"), json!({"type": SEVERING, "entity": {"type": "minecraft:cow"}, "result": "minecraft:leather", "per_level_chance": 1.5}))]).problems[0].contains("between 0 and 1"));
        assert!(Recipes::load(&[(id("t:bad"), json!({"type": "tconstruct:ageable_severing", "entity": {"type": "minecraft:cow"}}))]).problems[0].contains("adult_result"), "an ageable recipe without an adult result is a problem");
    }

    #[test]
    fn the_seven_shipped_stateful_recipes_load() {
        let loaded = shipped();
        assert_eq!((loaded.len(), loaded.problems.len()), (7, 0), "{:?}", loaded.problems);
        for entity in ["minecraft:chicken", "minecraft:rabbit", "minecraft:turtle", "minecraft:mooshroom", "minecraft:player", "minecraft:sheep", "minecraft:snow_golem"] {
            assert_eq!(loaded.find(&id(entity), &none).count(), 1, "{entity}");
        }
        let player = id("minecraft:player");
        let head = loaded.find(&player, &none).next().unwrap();
        assert!((head.chance(1.0, 0.0) - 0.1).abs() < 1e-6, "a beheading's shipped chances are 10% and 2%");
        assert!((head.chance(1.0, 1.0) - 0.12).abs() < 1e-6);
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
            None,
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
            None,
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
            None,
            &mut loot,
            &mut || 0.07,
        );
        assert!(loot.is_empty());
        let mut loot = Vec::new();
        loaded.process(
            &id("minecraft:creeper"),
            &none,
            true,
            1.0,
            0.0,
            None,
            &mut loot,
            &mut || 0.07,
        );
        assert_eq!(loot.len(), 1, "a rare mob's doubled chance (0.1) beats the 0.07 roll");
        // A head already among the loot ends it.
        let mut with_head = vec![pluto_lib::ItemSlot::new(id("minecraft:skeleton_skull"), 1)];
        loaded.process(
            &id("minecraft:creeper"),
            &none,
            true,
            20.0,
            0.0,
            None,
            &mut with_head,
            &mut || 0.0,
        );
        assert_eq!(with_head.len(), 1);
    }

    #[test]
    fn a_baby_drops_the_child_result_and_an_adult_the_adult_result() {
        let loaded = shipped();
        let baby = pluto_lib::LootEntityFacts { baby: true, ..facts() };
        // A baby turtle drops a scute, an adult the shell (a helmet).
        let loot = drops(&loaded, "minecraft:turtle", Some(&baby), &[]);
        assert_eq!(
            (loot.len(), loot[0].item.to_string(), loot[0].count),
            (1, "minecraft:scute".to_owned(), 1)
        );
        let loot = drops(&loaded, "minecraft:turtle", Some(&facts()), &[]);
        assert_eq!(loot[0].item.to_string(), "minecraft:turtle_helmet");
        // A baby chicken drops nothing (the recipe names no child result); an adult drops its feathers, up to the shipped two.
        assert!(drops(&loaded, "minecraft:chicken", Some(&baby), &[]).is_empty());
        let loot = drops(&loaded, "minecraft:chicken", Some(&facts()), &[0.99]);
        assert_eq!(
            (loot[0].item.to_string(), loot[0].count),
            ("minecraft:feather".to_owned(), 2)
        );
        // A baby rabbit drops nothing either.
        assert!(drops(&loaded, "minecraft:rabbit", Some(&baby), &[]).is_empty());
        let loot = drops(&loaded, "minecraft:rabbit", Some(&facts()), &[]);
        assert_eq!(loot[0].item.to_string(), "minecraft:rabbit_foot");
    }

    #[test]
    fn a_sheared_or_baby_sheep_drops_no_wool_and_a_grown_one_drops_its_colour() {
        let loaded = shipped();
        // Dye 14 is red: a grown red sheep that still has its wool drops red wool, up to the shipped two.
        let red = pluto_lib::LootEntityFacts { wool_color: Some(14), ..facts() };
        let loot = drops(&loaded, "minecraft:sheep", Some(&red), &[0.99]);
        assert_eq!(
            (loot[0].item.to_string(), loot[0].count),
            ("minecraft:red_wool".to_owned(), 2)
        );
        // No colour reported reads as white, the recipe's own base output.
        let loot = drops(&loaded, "minecraft:sheep", Some(&facts()), &[0.0]);
        assert_eq!(loot[0].item.to_string(), "minecraft:white_wool");
        let sheared = pluto_lib::LootEntityFacts { sheared: true, ..facts() };
        assert!(drops(&loaded, "minecraft:sheep", Some(&sheared), &[]).is_empty(), "a sheared sheep has no wool to sever");
        let baby = pluto_lib::LootEntityFacts { baby: true, ..facts() };
        assert!(drops(&loaded, "minecraft:sheep", Some(&baby), &[]).is_empty(), "a lamb drops no wool");
    }

    #[test]
    fn a_mooshroom_drops_five_of_its_own_mushrooms_and_a_baby_none() {
        let loaded = shipped();
        let brown = pluto_lib::LootEntityFacts { mooshroom: Some("brown".to_owned()), ..facts() };
        let loot = drops(&loaded, "minecraft:mooshroom", Some(&brown), &[0.99]);
        assert_eq!(
            (loot[0].item.to_string(), loot[0].count),
            ("minecraft:brown_mushroom".to_owned(), 5),
            "the count rolls up to the five the recipe drops"
        );
        let red = pluto_lib::LootEntityFacts { mooshroom: Some("red".to_owned()), ..facts() };
        let loot = drops(&loaded, "minecraft:mooshroom", Some(&red), &[0.0]);
        assert_eq!(
            (loot[0].item.to_string(), loot[0].count),
            ("minecraft:red_mushroom".to_owned(), 1)
        );
        let baby = pluto_lib::LootEntityFacts { baby: true, mooshroom: Some("brown".to_owned()), ..facts() };
        assert!(drops(&loaded, "minecraft:mooshroom", Some(&baby), &[]).is_empty());
    }

    #[test]
    fn a_snow_golem_drops_its_pumpkin_or_a_snow_block_without_it() {
        let loaded = shipped();
        let loot = drops(&loaded, "minecraft:snow_golem", Some(&facts()), &[]);
        assert_eq!(loot[0].item.to_string(), "minecraft:carved_pumpkin");
        let bare = pluto_lib::LootEntityFacts { sheared: true, ..facts() };
        let loot = drops(&loaded, "minecraft:snow_golem", Some(&bare), &[]);
        assert_eq!(loot[0].item.to_string(), "minecraft:snow_block");
    }

    #[test]
    fn a_players_head_keeps_their_profile() {
        let loaded = shipped();
        let uuid: u128 = 0x0123_4567_89ab_cdef_0123_4567_89ab_cdef;
        let beheaded = pluto_lib::LootEntityFacts {
            player: Some(pluto_lib::PlayerProfile { name: "Steve".to_owned(), uuid }),
            ..facts()
        };
        let loot = drops(&loaded, "minecraft:player", Some(&beheaded), &[]);
        assert_eq!(loot[0].item.to_string(), "minecraft:player_head");
        let profile = loot[0].profile.as_ref().expect("the head keeps the profile");
        assert_eq!(
            profile.get("name"),
            Some(&pluto_lib::Nbt::String("Steve".to_owned()))
        );
        assert_eq!(
            profile.get("id"),
            Some(&pluto_lib::Nbt::IntArray(vec![
                0x0123_4567,
                -0x7654_3211i32,
                0x0123_4567,
                -0x7654_3211i32,
            ])),
            "the uuid as its four ints, most significant first"
        );
        // No profile reported: the plain head the original drops for an entity that is not a player.
        let loot = drops(&loaded, "minecraft:player", None, &[]);
        assert_eq!(loot[0].item.to_string(), "minecraft:player_head");
        assert!(loot[0].profile.is_none());
    }

    #[test]
    fn stateful_recipes_drop_nothing_when_the_state_was_not_reported() {
        let loaded = shipped();
        for entity in ["minecraft:chicken", "minecraft:turtle", "minecraft:sheep", "minecraft:mooshroom", "minecraft:snow_golem"] {
            assert!(drops(&loaded, entity, None, &[]).is_empty(), "{entity}");
        }
    }
}
