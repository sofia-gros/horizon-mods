//! Advancement consumers, from SlimeKnights (MIT; see LICENSE): the item predicate type
//! `tconstruct:tool_stack` (ToolStackItemPredicate with the ToolStackPredicate loaders) and the
//! trigger `tconstruct:block_container_opened` (BlockContainerOpenedTrigger, fired by the
//! original's TriggeringBaseContainerMenu and TriggeringMultiModuleContainerMenu when a player
//! opens a block entity menu). The advancement data itself ships in the content package and
//! the game owns the tree, progress and rewards; this module owns only the Tinkers-shaped
//! evaluation, registered through PlutoLib's advancement API. Forms the original's loaders
//! accept but this port does not implement are rejected at decode, never treated as true.
use crate::{common, modifier_entries::List, tool_definitions, tool_materials::{MaterialList, MaterialVariant}, tool_stack};
use pluto_lib::advancement_api_v1::{PredicateRequest, PredicateResult, TagLookup};
use pluto_lib::{Identifier, Nbt, NbtCompound};
use serde_json::Value;

/// The item predicate type the shipped criteria name.
pub(crate) const TOOL_STACK: &str = "tconstruct:tool_stack";
/// The trigger id of BlockContainerOpenedTrigger.
pub(crate) const BLOCK_CONTAINER_OPENED: &str = "tconstruct:block_container_opened";
/// TinkerTags.Items.MODIFIABLE: the tag check comes first in ToolStackItemPredicate.matches,
/// so a stack that is not a modifiable tool never has its data read as a tool.
const MODIFIABLE: &str = "tconstruct:modifiable";

/// Registers the tool_stack item predicate. Called once from the product's hooks.
pub(crate) fn register() {
    let _ = pluto_lib::advancement_api_v1::register_item_predicate(pluto_lib::advancement_api_v1::PredicateRegistration {
        id: "tconstruct:advancements",
        callback: tool_stack_predicate,
        inventory_components: true,
    });
}

fn tool_stack_predicate(request: &PredicateRequest<'_>) -> PredicateResult {
    if request.predicate.get("type").and_then(Value::as_str) != Some(TOOL_STACK) {
        return PredicateResult::Unsupported;
    }
    if !request.tags.in_tag("item", MODIFIABLE, request.item.id) {
        return PredicateResult::Handled(false);
    }
    let Some(Nbt::Compound(custom_data)) = request.item.components else {
        return PredicateResult::Handled(false);
    };
    let Some(predicate) = request.predicate.get("predicate") else {
        return PredicateResult::Handled(false);
    };
    let decoded = match ToolPredicate::decode(predicate) {
        Ok(decoded) => decoded,
        Err(_) => return PredicateResult::Handled(false),
    };
    // The loaded registries only feed tool action checks; every other form is
    // stack-local, so an unloaded registry narrows tool_action to the tool
    // definition's own actions and never invents an answer.
    let item = Identifier::parse(request.item.id).ok();
    let definitions = tool_definitions::register().read().ok();
    let definition = match (&item, &definitions) {
        (Some(item), Some(definitions)) => definitions.data.get(item),
        _ => None,
    };
    let materials = common::register().read().ok();
    let modifiers = common::server_modifiers().and_then(|registry| registry.read().ok());
    let item_tags = common::item_snapshot();
    let environment = match (definition, &materials, &modifiers) {
        (Some(definition), Some(materials), Some(modifiers)) if materials.server_loaded && modifiers.definitions_loaded => {
            Some(tool_stack::Environment {
                definition: Some(definition),
                has_materials: definition.material_stat_types().is_some(),
                materials,
                modifiers,
                item_tags_loaded: item_tags.is_some(),
                item: item.as_ref(),
                item_tags: item_tags.as_deref(),
            })
        }
        _ => None,
    };
    let view = ToolView { item: request.item.id, custom_data, tags: request.tags, definition, environment: environment.as_ref() };
    PredicateResult::Handled(decoded.matches(&view))
}

/// BlockContainerOpenedTrigger.matches: the criterion's conditions name the block entity
/// type that must have been opened.
fn container_opened_matches(payload: &Value, conditions: &Value) -> bool {
    match (payload.get("type").and_then(Value::as_str), conditions.get("type").and_then(Value::as_str)) {
        (Some(opened), Some(expected)) => opened == expected,
        _ => false,
    }
}

/// A player opened the menu of a block entity the original registers under
/// `block_entity_type` (`tconstruct:smeltery` or `tconstruct:foundry` for the two heating
/// structure controllers). The port's controller block entities submit this from their use
/// hook, the boundary where the original's triggering menus are constructed; the submission
/// itself only queues the event for the native adapter, which binds it to the player.
pub(crate) fn container_opened(block_entity_type: &str) -> bool {
    pluto_lib::advancement_api_v1::submit_event(
        BLOCK_CONTAINER_OPENED,
        serde_json::json!({ "type": block_entity_type }),
        container_opened_matches,
    )
}

/// What a tool predicate may ask about: the stack's own saved data, its item, and — only for
/// tool actions — the loaded definition and registries behind it.
pub(crate) struct ToolView<'a> {
    pub item: &'a str,
    pub custom_data: &'a NbtCompound,
    pub tags: &'a dyn TagLookup,
    pub definition: Option<&'a tool_definitions::Data>,
    pub environment: Option<&'a tool_stack::Environment<'a>>,
}

impl ToolView<'_> {
    fn stat(&self, stat: &str) -> Option<&Nbt> {
        self.custom_data.get(tool_stack::TAG_STATS).and_then(Nbt::as_compound).and_then(|stats| stats.get(stat))
    }
    /// ModifierUtil.canPerformAction: the definition's own tool_actions module first,
    /// then every modifier's tool action hook. Without a loaded environment the modifier
    /// side cannot be answered and counts as not performing the action.
    fn can_perform_action(&self, action: &str) -> bool {
        if self.definition.is_some_and(|definition| definition.allows_action(action)) {
            return true;
        }
        self.environment.is_some_and(|env| tool_stack::can_perform_action(self.custom_data, env, action))
    }
}

/// Which saved modifier list HasModifierPredicate inspects (ModifierCheck).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ModifierCheck {
    Upgrades,
    All,
}

/// The tool predicate forms the original ships in advancement data, decoded from the
/// `predicate` field of a `tconstruct:tool_stack` item predicate. The string shorthand is
/// a registered singleton name, as in the original's FallbackPredicateRegistry.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ToolPredicate {
    /// The `tconstruct:has_upgrades` singleton: the upgrades list is not empty.
    HasUpgrades,
    HasModifier { modifier: Identifier, min: i32, max: i32, check: ModifierCheck },
    HasMaterial { variant: MaterialVariant, index: i32 },
    HasVolatileKey(Identifier),
    StatInRange { stat: String, min: f64, max: f64 },
    StatInSet { stat: String, values: Vec<Value> },
    ToolAction(String),
    /// mantle:set, an item predicate lifted into the tool registry: the tool's item is one of these.
    Set(Vec<Identifier>),
    /// mantle:tag, likewise lifted: the tool's item is in this item tag.
    Tag(Identifier),
    And(Vec<ToolPredicate>),
    Or(Vec<ToolPredicate>),
    Not(Box<ToolPredicate>),
}

fn id(value: Option<&Value>) -> Result<Identifier, String> {
    Identifier::parse(value.and_then(Value::as_str).ok_or("expected an identifier string")?).map_err(|e| e.to_string())
}

impl ToolPredicate {
    pub fn decode(value: &Value) -> Result<Self, String> {
        if let Some(name) = value.as_str() {
            return match name {
                "tconstruct:has_upgrades" => Ok(Self::HasUpgrades),
                other => Err(format!("unregistered tool predicate {other}")),
            };
        }
        let object = value.as_object().ok_or("tool predicate must be an object or a singleton name")?;
        Self::decode_typed(object, object.get("type").ok_or("tool predicate type missing")?)
    }

    fn decode_typed(object: &serde_json::Map<String, Value>, kind: &Value) -> Result<Self, String> {
        let kind = kind.as_str().ok_or("tool predicate type must be a string")?;
        let list = |key: &str| -> Result<Vec<ToolPredicate>, String> {
            object
                .get(key)
                .and_then(Value::as_array)
                .filter(|values| !values.is_empty())
                .ok_or_else(|| format!("{key} must be a nonempty array"))?
                .iter()
                .map(ToolPredicate::decode)
                .collect()
        };
        Ok(match kind {
            "tconstruct:has_modifier" => {
                let matcher = object.get("modifier").and_then(Value::as_object).ok_or("modifier must be an object")?;
                if matcher.get("type").and_then(Value::as_str) != Some("tconstruct:single") {
                    return Err("only single-modifier matches are implemented".into());
                }
                let (min, max) = match object.get("level") {
                    None | Some(Value::Null) => (1, 32767),
                    Some(Value::Number(n)) => {
                        let n = crate::materials::number_int(&n.to_string());
                        (n, n)
                    }
                    Some(Value::Object(range)) => (
                        range.get("min").and_then(Value::as_i64).map_or(1, |n| n as i32),
                        range.get("max").and_then(Value::as_i64).map_or(32767, |n| n as i32),
                    ),
                    _ => return Err("level must be a number or range".into()),
                };
                let check = match object.get("check").and_then(Value::as_str) {
                    Some("upgrades") => ModifierCheck::Upgrades,
                    Some("all") => ModifierCheck::All,
                    _ => return Err("check must be all or upgrades".into()),
                };
                Self::HasModifier { modifier: id(matcher.get("modifier"))?, min, max, check }
            }
            "tconstruct:has_material" => {
                let material = object.get("material").and_then(Value::as_str).ok_or("material must be a variant string")?;
                let variant = MaterialVariant::parse(material).ok_or_else(|| format!("invalid material variant {material}"))?;
                let index = match object.get("index") {
                    None | Some(Value::Null) => -1,
                    Some(Value::Number(n)) => crate::materials::number_int(&n.to_string()),
                    _ => return Err("index must be a number".into()),
                };
                Self::HasMaterial { variant, index }
            }
            "tconstruct:has_volatile_key" => Self::HasVolatileKey(id(object.get("key"))?),
            "tconstruct:tool_action" => {
                Self::ToolAction(object.get("action").and_then(Value::as_str).ok_or("action must be a string")?.to_owned())
            }
            "tconstruct:stat_in_range" => Self::StatInRange {
                stat: object.get("stat").and_then(Value::as_str).ok_or("stat must be a string")?.to_owned(),
                min: object.get("min").and_then(Value::as_f64).unwrap_or(f64::NEG_INFINITY),
                max: object.get("max").and_then(Value::as_f64).unwrap_or(f64::INFINITY),
            },
            "tconstruct:stat_in_set" => Self::StatInSet {
                stat: object.get("stat").and_then(Value::as_str).ok_or("stat must be a string")?.to_owned(),
                values: object.get("values").and_then(Value::as_array).ok_or("values must be an array")?.clone(),
            },
            "mantle:and" => Self::And(list("predicates")?),
            "mantle:or" => Self::Or(list("predicates")?),
            "mantle:set" => Self::Set(
                object
                    .get("items")
                    .and_then(Value::as_array)
                    .ok_or("items must be an array")?
                    .iter()
                    .map(|item| id(Some(item)))
                    .collect::<Result<_, _>>()?,
            ),
            "mantle:tag" => Self::Tag(id(object.get("tag"))?),
            "mantle:inverted" => {
                // The wrapped predicate's own fields sit beside inverted_type.
                let inner = object.get("inverted_type").ok_or("inverted_type missing")?;
                if inner.as_str() == Some("mantle:inverted") {
                    return Err("nested inversion is not shipped".into());
                }
                Self::Not(Box::new(Self::decode_typed(object, inner)?))
            }
            other => return Err(format!("unimplemented tool predicate {other}")),
        })
    }

    pub fn matches(&self, view: &ToolView<'_>) -> bool {
        match self {
            Self::HasUpgrades => !List::from_nbt(view.custom_data.get(tool_stack::TAG_UPGRADES)).entries().is_empty(),
            Self::HasModifier { modifier, min, max, check } => {
                let key = match check {
                    ModifierCheck::Upgrades => tool_stack::TAG_UPGRADES,
                    ModifierCheck::All => tool_stack::TAG_MODIFIERS,
                };
                let level: i32 = List::from_nbt(view.custom_data.get(key))
                    .entries()
                    .iter()
                    .filter(|entry| &entry.id == modifier)
                    .map(|entry| entry.int_effective_level())
                    .sum();
                // Level 0 means absent; the full valid range means present at any level.
                if *min == 0 && *max == 0 {
                    level == 0
                } else if (*min, *max) == (1, 32767) {
                    level > 0
                } else {
                    (*min..=*max).contains(&level)
                }
            }
            Self::HasMaterial { variant, index } => {
                let list = MaterialList::from_nbt(view.custom_data.get(tool_stack::TAG_MATERIALS));
                let wanted = variant.normalized();
                if *index >= 0 {
                    list.get(*index).normalized() == wanted
                } else {
                    list.as_slice().iter().any(|held| held.normalized() == wanted)
                }
            }
            Self::HasVolatileKey(key) => view
                .custom_data
                .get(tool_stack::TAG_VOLATILE)
                .and_then(Nbt::as_compound)
                .is_some_and(|volatile| volatile.contains_key(&key.to_string())),
            Self::StatInRange { stat, min, max } => {
                // A stat the stack never recorded reads as the stat's default;
                // the numeric stats' defaults are 0.
                let value = view.stat(stat).and_then(Nbt::as_f64).unwrap_or(0.0);
                value >= *min && value <= *max
            }
            Self::StatInSet { stat, values } => match view.stat(stat) {
                Some(Nbt::String(text)) => values.iter().any(|value| value.as_str() == Some(text.as_str())),
                Some(number) => {
                    let number = number.as_f64();
                    number.is_some_and(|number| values.iter().any(|value| value.as_f64() == Some(number)))
                }
                _ => false,
            },
            Self::ToolAction(action) => view.can_perform_action(action),
            Self::Set(items) => items.iter().any(|item| item.to_string() == view.item),
            Self::Tag(tag) => view.tags.in_tag("item", &tag.to_string(), view.item),
            Self::And(all) => all.iter().all(|predicate| predicate.matches(view)),
            Self::Or(any) => any.iter().any(|predicate| predicate.matches(view)),
            Self::Not(inner) => !inner.matches(view),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::advancement_api_v1::ItemSnapshot;
    use serde_json::json;

    struct Tags;
    impl TagLookup for Tags {
        fn in_tag(&self, registry: &str, tag: &str, id: &str) -> bool {
            registry == "item" && tag == "tconstruct:modifiable" && id == "tconstruct:pickaxe"
                || registry == "item" && tag == "tconstruct:modifiable/harvest" && id == "tconstruct:pickaxe"
        }
    }

    fn tool_data() -> NbtCompound {
        let mut stats = NbtCompound::new();
        stats.insert("tconstruct:harvest_tier".into(), Nbt::String("minecraft:iron".into()));
        stats.insert("tconstruct:attack_damage".into(), Nbt::Float(21.5));
        let mut volatile = NbtCompound::new();
        volatile.insert("tconstruct:snow_boots".into(), Nbt::Byte(1));
        let mut data = NbtCompound::new();
        data.insert(
            tool_stack::TAG_MATERIALS.into(),
            Nbt::List(vec![Nbt::String("tconstruct:iron".into()), Nbt::String("tconstruct:wood".into())]),
        );
        data.insert(
            tool_stack::TAG_UPGRADES.into(),
            Nbt::List(vec![Nbt::Compound(
                crate::modifier_entries::Entry::new(Identifier::parse("tconstruct:haste").unwrap(), 2).to_nbt(),
            )]),
        );
        data.insert(tool_stack::TAG_STATS.into(), Nbt::Compound(stats));
        data.insert(tool_stack::TAG_VOLATILE.into(), Nbt::Compound(volatile));
        data
    }

    fn make_view<'a>(data: &'a NbtCompound, definition: Option<&'a tool_definitions::Data>) -> ToolView<'a> {
        ToolView { item: "tconstruct:pickaxe", custom_data: data, tags: &Tags, definition, environment: None }
    }

    fn decode(value: Value) -> ToolPredicate {
        ToolPredicate::decode(&value).unwrap()
    }

    #[test]
    fn shipped_forms_evaluate_over_the_saved_tool_data() {
        let data = tool_data();
        let view = make_view(&data, None);
        assert!(decode(json!("tconstruct:has_upgrades")).matches(&view));
        let has = |check: &str, modifier: &str| {
            decode(json!({"type": "tconstruct:has_modifier", "check": check, "modifier": {"type": "tconstruct:single", "modifier": modifier}}))
        };
        assert!(has("upgrades", "tconstruct:haste").matches(&view));
        assert!(!has("all", "tconstruct:haste").matches(&view), "the final modifier list was never saved on this stack");
        assert!(!has("upgrades", "tconstruct:sharpness").matches(&view));
        let material = |value: Value| decode(value);
        assert!(material(json!({"type": "tconstruct:has_material", "material": "tconstruct:iron"})).matches(&view));
        assert!(material(json!({"type": "tconstruct:has_material", "material": "tconstruct:wood", "index": 1})).matches(&view));
        assert!(!material(json!({"type": "tconstruct:has_material", "material": "tconstruct:wood", "index": 0})).matches(&view));
        assert!(!material(json!({"type": "tconstruct:has_material", "material": "tconstruct:cobalt"})).matches(&view));
        assert!(decode(json!({"type": "tconstruct:has_volatile_key", "key": "tconstruct:snow_boots"})).matches(&view));
        assert!(!decode(json!({"type": "tconstruct:has_volatile_key", "key": "tconstruct:shiny"})).matches(&view));
        assert!(decode(json!({"type": "tconstruct:stat_in_range", "stat": "tconstruct:attack_damage", "min": 20.0})).matches(&view));
        assert!(!decode(json!({"type": "tconstruct:stat_in_range", "stat": "tconstruct:attack_damage", "min": 22.0})).matches(&view));
        assert!(decode(json!({"type": "tconstruct:stat_in_set", "stat": "tconstruct:harvest_tier", "values": ["minecraft:iron"]})).matches(&view));
        assert!(!decode(json!({"type": "tconstruct:stat_in_set", "stat": "tconstruct:harvest_tier", "values": ["minecraft:diamond"]})).matches(&view));
        assert!(decode(json!({"type": "mantle:tag", "tag": "tconstruct:modifiable/harvest"})).matches(&view));
        assert!(decode(json!({"type": "mantle:set", "items": ["tconstruct:pickaxe"]})).matches(&view));
        assert!(!decode(json!({"type": "mantle:set", "items": ["tconstruct:axe"]})).matches(&view));
    }

    #[test]
    fn the_internal_pickaxe_composition_needs_every_part() {
        let data = tool_data();
        let definition = tool_definitions::Data::decode(&json!({"modules": [
            {"type": "tconstruct:tool_actions", "tool_actions": ["pickaxe_dig"]}
        ]}))
        .unwrap();
        let view = make_view(&data, Some(&definition));
        let composite = decode(json!({"type": "mantle:and", "predicates": [
            {"type": "mantle:tag", "tag": "tconstruct:modifiable/harvest"},
            {"type": "tconstruct:tool_action", "action": "pickaxe_dig"},
            {"type": "tconstruct:stat_in_set", "stat": "tconstruct:harvest_tier", "values": ["minecraft:iron"]}
        ]}));
        assert!(composite.matches(&view));
        // Without the definition the action cannot be answered and fails closed.
        assert!(!composite.matches(&make_view(&data, None)));
    }

    #[test]
    fn unimplemented_and_malformed_forms_are_rejected_never_true() {
        for value in [
            json!("tconstruct:not_broken"),
            json!({"type": "tconstruct:has_persistent_key", "key": "tconstruct:x"}),
            json!({"type": "tconstruct:has_modifier", "check": "all", "modifier": {"type": "tconstruct:tag", "tag": "tconstruct:x"}}),
            json!({"type": "tconstruct:has_modifier", "modifier": {"type": "tconstruct:single", "modifier": "tconstruct:x"}}),
            json!({"type": "tconstruct:has_material", "material": {"type": "mantle:tag"}}),
            json!(42),
        ] {
            assert!(ToolPredicate::decode(&value).is_err(), "{value}");
        }
    }

    #[test]
    fn the_callback_owns_only_tool_stack_and_checks_the_modifiable_tag_first() {
        let predicate = json!({"type": "tconstruct:tool_stack", "predicate": "tconstruct:has_upgrades"});
        let snapshot = ItemSnapshot { id: "minecraft:stick", count: 1, damage: 0, enchantments: &[], potion: None, custom_name: false, components: None };
        let request = PredicateRequest { predicate: &predicate, item: snapshot, tags: &Tags };
        assert_eq!(tool_stack_predicate(&request), PredicateResult::Handled(false), "not a modifiable tool");
        let other = json!({"items": ["minecraft:stick"]});
        let request = PredicateRequest { predicate: &other, item: snapshot, tags: &Tags };
        assert_eq!(tool_stack_predicate(&request), PredicateResult::Unsupported);
        // A modifiable tool with no saved data has no upgrades.
        let data = NbtCompound::new();
        let components = Nbt::Compound(data);
        let snapshot = ItemSnapshot { id: "tconstruct:pickaxe", count: 1, damage: 0, enchantments: &[], potion: None, custom_name: false, components: Some(&components) };
        let request = PredicateRequest { predicate: &predicate, item: snapshot, tags: &Tags };
        assert_eq!(tool_stack_predicate(&request), PredicateResult::Handled(false));
    }

    #[test]
    fn container_opened_submits_an_event_the_criterion_matcher_decides() {
        let _ = pluto_lib::advancement_api_v1::take_queued_events();
        assert!(container_opened("tconstruct:smeltery"));
        let queued = pluto_lib::advancement_api_v1::take_queued_events();
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].trigger, BLOCK_CONTAINER_OPENED);
        assert!((queued[0].matches)(&queued[0].payload, &json!({"type": "tconstruct:smeltery"})));
        assert!(!(queued[0].matches)(&queued[0].payload, &json!({"type": "tconstruct:foundry"})));
    }
}
