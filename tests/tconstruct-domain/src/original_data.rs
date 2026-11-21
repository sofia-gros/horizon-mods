//! The original mod's generated data, as copied into the content root, through the production
//! decoders. This is the check that decoders accept what the original actually ships; synthetic
//! fixtures elsewhere only cover the shapes their authors thought of.
use crate::{materials::Definitions as Materials, modifiers::Registry, tool_definitions::Data};
use pluto_lib::{Identifier, JsonResourceStack};
use serde_json::Value;
use std::path::PathBuf;

pub(crate) fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content/data/tconstruct/tinkering")
}
pub(crate) fn files(folder: &str) -> Vec<(Identifier, Value)> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(root().join(folder)).expect("content folder") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") { continue; }
        let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
        let value = serde_json::from_slice(&std::fs::read(&path).unwrap()).expect("valid JSON");
        found.push((Identifier::new("tconstruct", &name).unwrap(), value));
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}
pub(crate) fn stacks(folder: &str) -> Vec<JsonResourceStack> {
    files(folder).into_iter().map(|(id, value)| JsonResourceStack { id, layers: vec![Ok(value)] }).collect()
}
/// The production condition tree over an item registry with no external ingot tags and default config,
/// the vanilla-only world the other-mod materials were excluded for.
pub(crate) fn conditions() -> crate::conditions::Context {
    crate::conditions::Context { items: Some(pluto_lib::fixtures::registry(&[]).unwrap()), config: crate::config::parse("").unwrap() }
}

#[test]
fn smeltery_damage_types_resolve_with_the_original_effects_and_tags() {
    let types = files("../damage_type");
    for (name, is_fire) in [("smeltery_heat", true), ("smeltery_magic", false)] {
        let id = Identifier::new("tconstruct", name).unwrap();
        let (_, definition) = types.iter().find(|(key, _)| key == &id).expect("smeltery damage type");
        assert_eq!(*definition, serde_json::json!({
            "effects": "burning", "exhaustion": 0.1, "message_id": format!("tconstruct.{name}"), "scaling": "never"
        }));
        for (tag, expected) in [("is_fire", is_fire), ("witch_resistant_to", !is_fire), ("bypasses_armor", false)] {
            let path = root().join(format!("../../minecraft/tags/damage_type/{tag}.json"));
            let data: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
            assert_eq!(data["values"].as_array().unwrap().contains(&Value::String(id.to_string())), expected, "{id} in {tag}");
        }
    }
}

#[test]
fn every_original_damage_type_is_shipped_with_its_full_vanilla_tag_memberships() {
    let types = files("../damage_type");
    assert_eq!(types.len(), 32, "all 32 original damage types");
    for (id, definition) in &types {
        // Every definition keeps the original's shape: a message id, a scaling and an exhaustion.
        assert!(definition["message_id"].is_string(), "{id} message id");
        assert!(["never", "always", "when_caused_by_living_non_player"].contains(&definition["scaling"].as_str().unwrap_or_default()), "{id} scaling");
        assert!(definition["exhaustion"].is_number(), "{id} exhaustion");
    }
    // The port's additions to vanilla's damage-type tags name exactly the original's memberships;
    // they were trimmed to the imported types while only some types were shipped, and are whole now.
    let memberships: &[(&str, &[&str])] = &[
        ("avoids_guardian_thorns", &["bleeding", "shock"]),
        ("bypasses_armor", &["piercing", "self_destruct", "bleeding", "entangled", "spiny", "update_health", "water_melee", "water_ranged", "fluid_spike_melee", "fluid_spike_ranged"]),
        ("bypasses_cooldown", &["update_health"]),
        ("bypasses_effects", &["entangled", "spiny", "update_health"]),
        ("bypasses_enchantments", &["bleeding"]),
        ("is_explosion", &["self_destruct", "explosion_melee", "explosion_ranged", "mob_explosion_melee", "mob_explosion_ranged"]),
        ("is_fire", &["smeltery_heat", "fluid_fire_melee", "fluid_fire_ranged"]),
        ("is_freezing", &["fluid_cold_melee", "fluid_cold_ranged"]),
        ("is_projectile", &["thrown_tool", "fishing_hook", "fluid_impact_ranged", "fluid_fire_ranged", "fluid_cold_ranged", "fluid_magic_ranged", "water_ranged", "fluid_spike_ranged", "explosion_ranged", "mob_explosion_ranged"]),
        ("witch_resistant_to", &["smeltery_magic", "fluid_magic_melee", "fluid_magic_ranged"]),
    ];
    for (tag, names) in memberships {
        let path = root().join(format!("../../minecraft/tags/damage_type/{tag}.json"));
        let data: Value = serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|_| panic!("{tag} tag file"))).unwrap();
        let values: Vec<String> = data["values"].as_array().unwrap().iter().map(|value| value.as_str().unwrap().to_owned()).collect();
        let expected: Vec<String> = names.iter().map(|name| format!("tconstruct:{name}")).collect();
        assert_eq!(values, expected, "{tag} membership");
    }
}

#[test]
fn every_original_trim_material_is_shipped_as_data() {
    // The trim materials ship as data, so the registry entries exist. The trim *rendering* consumer is
    // a separate modifier-side gap (the trim modifier kind has no rendering hook), so this test pins
    // the data only and decodes nothing into behaviour.
    let materials = files("../trim_material");
    assert_eq!(materials.len(), 16, "all 16 original trim materials");
    for (id, material) in &materials {
        assert_eq!(material["asset_name"], format!("tconstruct_{}", id.path()), "{id} asset name");
        assert!(material["ingredient"].as_str().is_some_and(|ingredient| ingredient.starts_with("tconstruct:")), "{id} ingredient");
        assert!(material["item_model_index"].is_number(), "{id} item model index");
        assert!(material["description"]["translate"].is_string(), "{id} description");
    }
}

#[test]
fn every_shipped_tool_definition_decodes() {
    let definitions = files("tool_definitions");
    assert_eq!(definitions.len(), 44, "45 original definitions minus the other-mod minotaur axe");
    let failures: Vec<String> = definitions.iter()
        .filter_map(|(id, value)| Data::decode(value).err().map(|error| format!("{id}: {error}"))).collect();
    assert!(failures.is_empty(), "definitions that do not decode: {failures:#?}");
}

#[test]
fn every_shipped_material_loads_with_stats_and_traits_without_problems() {
    let mut registry = Materials::default();
    let definitions = files("materials/definition");
    assert_eq!(definitions.len(), 104, "every original material definition ships, the sixteen integration materials included");
    registry = Materials::load(&definitions, &conditions());
    assert!(registry.problems.is_empty(), "material definition problems: {:#?}", registry.problems);
    // The sixteen integration materials are gated on another mod's ingot tag (or the
    // force_integration_materials config), so in this vanilla-only context they select
    // nothing: 88 of the 104 definitions load, exactly as in Java without those mods.
    assert_eq!(registry.materials.len() + registry.redirects.len(), 88, "the sixteen integration materials stay unselected while their ingot tags are empty");
    for name in ["aluminum", "bronze", "constantan", "electrum", "fiery", "invar", "ironwood", "lead", "necronium", "nicrosil", "osmium", "pewter", "plated_slimewood", "silver", "steeleaf", "treated_wood"] {
        let id = Identifier::new("tconstruct", name).unwrap();
        assert!(!registry.materials.contains_key(&id) && !registry.redirects.contains_key(&id), "{name} is an integration material and must not select here");
    }
    let mut problems = Vec::new();
    registry.stats = crate::material_stats::load(&stacks("materials/stats"), &mut problems);
    registry.traits = crate::material_traits::load(&stacks("materials/traits"), &mut problems);
    assert!(problems.is_empty(), "material stats/traits problems: {problems:#?}");
    assert_eq!(registry.stats.len(), 99, "every concrete material ships stats, the sixteen integration materials included");
    assert_eq!(registry.traits.len(), 99, "every concrete material ships traits, the sixteen integration materials included");
    assert!(registry.materials.keys().all(|id| registry.stats.contains_key(id)), "every loaded material ships stats");
}

#[test]
fn shipped_modifiers_report_exactly_the_modules_that_are_not_ported() {
    let mut registry = Registry::default();
    registry.reload(&files("modifiers"), &conditions());
    assert!(registry.problems.is_empty(), "modifiers that do not decode: {:#?}", registry.problems);
    let unbound = (0..).zip(registry.dynamic_ids()).filter(|(_, id)| registry.get(id).is_some_and(|modifier| !modifier.unbound.is_empty())).count();
    let mut kinds = std::collections::BTreeSet::new();
    for id in registry.dynamic_ids() { if let Some(modifier) = registry.get(id) { for kind in &modifier.unbound { kinds.insert(kind.to_string()); } } }
    // Modifiers with at least one unported module; it only shrinks as modules are ported.
    assert_eq!((unbound, kinds.len()), (98, 65), "modifiers carrying unported modules, and the kinds: {kinds:?}");
    // Presentation modules and tool actions are bound, and show_offhand counts into the armor level totals over worn pieces; the boot walk transform (no walk event) is not.
    for bound in ["show_interaction_source", "variant_color", "variant_name", "material_variant_color", "durability_color", "stat_tooltip", "tool_actions", "mob_effect", "armor_attack_mob_effect", "fiery_attack", "fiery_counter", "freezing_attack", "freezing_counter", "knockback_counter", "tool_usage_mob_effect", "enchantment_ignoring_protection", "clear_effect_on_unequip", "mining_capacity", "loot_to_capacity"] { assert!(!kinds.contains(&format!("tconstruct:{bound}")), "{bound} should be bound"); }
    // Restore lost health bound to the modify hurt hook runs as the damage arrives, before armor, and stalwart's protection
    // reads the equipment the attacker holds, which the hit reports: both bind now. Recurrent protection, the other module
    // that works before armor, is a kind of its own and stays unbound.
    for unbound in ["tool_action_walk_transform", "embellishment", "recurrent_protection"] { assert!(kinds.contains(&format!("tconstruct:{unbound}")), "{unbound} is still unbound"); }
    for bound in ["restore_lost_health", "protection"] { assert!(!kinds.contains(&format!("tconstruct:{bound}")), "{bound} should be bound"); }
    for bound in ["depth_protection", "overshield", "damage_on_unequip", "reduce_effect_on_unequip", "reduce_tool_damage", "lifesteal", "launch_capacity"] { assert!(!kinds.contains(&format!("tconstruct:{bound}")), "{bound} should be bound"); }
    // The craft count, material repair, volatile float and quiver inventory modules are bound; revitalizing's
    // update health binds for its armor slots but its hand module stays unbound (held tools are not reported).
    for bound in ["craft_count", "material_repair", "volatile_float", "quiver_inventory"] { assert!(!kinds.contains(&format!("tconstruct:{bound}")), "{bound} should be bound"); }
    assert!(kinds.contains("tconstruct:update_health"), "update_health is still unbound for the hand slots");
    let unbound_of = |name: &str| registry.get(&Identifier::parse(&format!("tconstruct:{name}")).unwrap()).unwrap().unbound.iter().map(|kind| kind.to_string()).collect::<Vec<_>>();
    assert!(unbound_of("stalwart").is_empty(), "stalwart's protection counts the attacker's filled equipment slots, which the hit reports");
    assert!(unbound_of("restore").is_empty(), "restore runs on the modify hurt hook, before armor");
    // The mining speed forms over the facts of the dig bind: temperate reads the temperature of the biome at the block and
    // lightspeed the block light. The forms over a block property (blasting's blast resistance, crumbling's requires-tool)
    // and over the holder as an entity (hydraulic) stay unbound; lightspeed keeps only its own attribute kind.
    assert!(unbound_of("temperate").is_empty(), "temperate is fully bound");
    assert_eq!(unbound_of("lightspeed"), ["tconstruct:lightspeed_attribute"]);
    assert_eq!(unbound_of("lightspeed_armor"), ["tconstruct:lightspeed_attribute"]);
    for name in ["blasting", "crumbling", "hydraulic"] { assert_eq!(unbound_of(name), ["tconstruct:conditional_mining_speed"], "{name}"); }
    for name in ["cheap", "economical", "stringy"] { assert!(unbound_of(name).is_empty(), "{name} is fully bound"); }
    assert_eq!(unbound_of("revitalizing"), ["tconstruct:update_health"], "the armor update health binds; the hand one is not reported");
    assert!(unbound_of("rebuff").is_empty(), "the same module on the modify damage hook is bound");
    // Armor looting is bound: a worn piece's modules add into the looting total the severing flow captures from the killer's hit
    // (ArmorLootingModifierHook.getLooting, the total SeveringModule reads). Armor harvest enchantments stay unbound: no break
    // flow reports the breaker's worn equipment or lets a module change the enchantments a block's drops roll with, so the merge
    // the original performs on the breaking stack before the roll has no consumer here.
    assert!(!kinds.contains("tconstruct:armor_looting"), "armor_looting should be bound");
    assert!(kinds.contains("tconstruct:armor_harvest_enchantment"), "armor_harvest_enchantment is still unbound");
    assert!(unbound_of("fortunate").is_empty(), "fortunate is fully bound");
    assert!(unbound_of("looting").is_empty(), "looting is fully bound");
    assert_eq!(unbound_of("looter"), ["tconstruct:attribute"], "only looter's experience multiplier attribute stays unbound");
    assert_eq!(unbound_of("luck"), ["tconstruct:armor_harvest_enchantment"], "luck keeps only its armor harvest enchantment unbound");
    assert_eq!(unbound_of("fortune"), ["tconstruct:armor_harvest_enchantment"]);
    assert_eq!(unbound_of("silky"), ["tconstruct:armor_harvest_enchantment"]);
    for name in ["vital_protection", "loyal", "nightmare_protection", "flamestance", "invariant", "solar_powered", "entangled", "entwined", "depth_protection", "overshield", "stripping", "sweeping_edge"] { assert!(unbound_of(name).is_empty(), "{name} is fully bound"); }
    // The block-break modules are bound (mining and loot capacity, the break half of tool usage effects); autosmelt and smelting need the furnace recipes, which PlutoLib does not expose.
    // The edible family is bound but for its representative item (item particles, eating sounds and the Diet integration while the use runs).
    for bound in ["edible", "edible_consume_durability", "edible_cure_random_effect", "edible_remove_effect"] { assert!(!kinds.contains(&format!("tconstruct:{bound}")), "{bound} should be bound"); }
    // The special melee consumers are bound: punch and reverse punch over the projectile hooks, the melee instrument, the
    // overgrowth inventory tick and kinetic's insatiable feed. Smashing runs on the fluid effects engine, which is not
    // ported, and overburn still carries its tank stat boost (and its own module needs a tool fluid tank).
    for bound in ["punch", "reverse_punch", "melee_instrument", "overgrowth", "kinetic"] { assert!(!kinds.contains(&format!("tconstruct:{bound}")), "{bound} should be bound"); }
    for name in ["punch", "drawback", "kinetic", "overgrowth", "ram_attack"] { assert!(unbound_of(name).is_empty(), "{name} is fully bound"); }
    for unbound in ["smashing", "overburn"] { assert!(kinds.contains(&format!("tconstruct:{unbound}")), "{unbound} is still unbound"); }
    assert!(!unbound_of("overburn").is_empty(), "overburn still carries unported modules");
    assert!(!unbound_of("smashing").is_empty(), "smashing still carries its unported module");
    // The inventory consumers are bound: the quivers draw through the ammo event, the strap and belt
    // kinds decode with their interact plan, share durability runs on the wear event, and shears and
    // fishing answer the tool-action query. Harvest stays unbound (no destroy-with-drops or block-use
    // re-dispatch reaches a use callback), as does the armor harvest enchantment above.
    for bound in ["bulk_quiver", "trick_quiver", "shield_strap", "tool_belt", "share_durability", "shears", "fishing"] { assert!(!kinds.contains(&format!("tconstruct:{bound}")), "{bound} should be bound"); }
    assert!(kinds.contains("tconstruct:harvest"), "harvest is still unbound");
    for name in ["bulk_quiver", "trick_quiver", "sliver", "deciduous", "fishing", "shears", "silky_shears", "silky_aoe_shears", "shield_strap", "tool_belt"] { assert!(unbound_of(name).is_empty(), "{name} is fully bound"); }
}

#[test]
fn the_shipped_special_melee_modules_run_on_a_tool() {
    use crate::{modifier_entries::List, tool_stack::{kinetic_level, melee_instrument_sound, overgrowth_tick, punch_shoot, reverse_punch_push, Environment, TAG_MATERIALS, TAG_MODIFIERS, TAG_STATS}};
    use pluto_lib::{Nbt, NbtCompound};
    let world = world();
    let env = Environment { definition: None, has_materials: true, materials: &world.materials, modifiers: &world.registry, item_tags_loaded: true, item: None, item_tags: None };
    let tool = |modifier: &str, level: i32| {
        let mut nbt = NbtCompound::new();
        nbt.insert(TAG_MODIFIERS.into(), List::default().with_modifier(Identifier::parse(modifier).unwrap(), level).unwrap().to_nbt());
        nbt
    };
    // Punch sets an arrow's knockback to the amount (1 per level in the shipped data), and adds it to a knockback projectile.
    assert_eq!(punch_shoot(&tool("tconstruct:punch", 2), &env, true), 2.0);
    assert_eq!(punch_shoot(&tool("tconstruct:punch", 2), &env, false), 2.0);
    // Drawback's reverse punch pulls the target back along the shot's line: (0.6 + 0.8) * (1 - 0.25) = 1.05 over (3, 4).
    let push = reverse_punch_push(&tool("tconstruct:drawback", 1), &env, true, 0.25, [3.0, 4.0]).unwrap();
    assert!((push[0] - -0.63).abs() < 1e-6 && (push[1] - 0.1).abs() < 1e-6 && (push[2] - -0.84).abs() < 1e-6, "{push:?}");
    // Ram attack plays the singing goat horn its horn material names, for a sprinting attacker only.
    let mut ram = tool("tconstruct:ram_attack", 1);
    ram.insert(TAG_MATERIALS.into(), Nbt::List(vec![Nbt::String("tconstruct:horn#minecraft.sing_goat_horn".into())]));
    let sprinting = |tag: &Identifier| tag.to_string() == "tconstruct_state:attacker/sprinting";
    let sounds = melee_instrument_sound(&ram, &env, &sprinting, &|_| true, 0.0);
    assert_eq!(sounds.len(), 1);
    assert_eq!((sounds[0].event.as_str(), sounds[0].volume), ("minecraft:item.goat_horn.sound.1", 16.0));
    assert!(melee_instrument_sound(&ram, &env, &|_| false, &|_| true, 0.0).is_empty(), "a standing attacker plays nothing");
    // Overgrowth grows a point of overslime on the second's tick when the roll lands, and only then.
    let mut over = tool("tconstruct:overgrowth", 1);
    let mut stats = NbtCompound::new();
    stats.insert("tconstruct:overslime".into(), Nbt::Float(10.0));
    over.insert(TAG_STATS.into(), Nbt::Compound(stats));
    assert!(overgrowth_tick(&mut over, &env, 20, false, &mut || 0.0));
    assert_eq!(crate::overslime::amount(&over), 1);
    assert!(!overgrowth_tick(&mut over.clone(), &env, 21, false, &mut || 0.0), "only every twentieth tick");
    assert!(!overgrowth_tick(&mut over.clone(), &env, 20, true, &mut || 0.0), "not while the holder uses the stack");
    // Kinetic totals the plain levels of its entries on a piece; a broken piece contributes nothing.
    assert_eq!(kinetic_level(&tool("tconstruct:kinetic", 2), &env), 2);
    let mut broken = tool("tconstruct:kinetic", 2);
    broken.insert(crate::tool_stack::TAG_BROKEN.into(), Nbt::Byte(1));
    assert_eq!(kinetic_level(&broken, &env), 0);
}

#[test]
fn shipped_armor_looting_adds_its_level_to_the_wearers_kills_in_the_listed_slots() {
    use crate::{modifier_entries::List, tool_stack::{armor_looting, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{Nbt, NbtCompound};
    let world = world();
    let helmet = &world.tools.iter().find(|(id, _)| id.path() == "plate_helmet").unwrap().1;
    let helmet_id = Identifier::parse("tconstruct:plate_helmet").unwrap();
    let armor_tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:plate_helmet"], &[("tconstruct:modifiable/armor", &["tconstruct:plate_helmet"]), ("tconstruct:modifiable/armor/worn", &["tconstruct:plate_helmet"])]).unwrap();
    let env = Environment { definition: Some(helmet), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&helmet_id), item_tags: Some(&armor_tags) };
    let piece = |upgrades: &[(&str, i32)]| {
        let mut nbt = NbtCompound::new();
        nbt.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:cobalt", "tconstruct:iron"].map(|m| Nbt::String(m.into())).to_vec()));
        let mut list = List::default();
        for (id, level) in upgrades { list = list.with_modifier(Identifier::parse(id).unwrap(), *level).unwrap(); }
        nbt.insert(TAG_UPGRADES.into(), list.to_nbt());
        assert!(rebuild_stats(&mut nbt, &env).unwrap());
        nbt
    };
    // looting I lists all four armor slots at one per level; a hand is not one of them.
    let looting = piece(&[("tconstruct:looting", 1)]);
    assert_eq!(armor_looting(&looting, &env, "head"), 1);
    assert_eq!(armor_looting(&looting, &env, "feet"), 1);
    assert_eq!(armor_looting(&looting, &env, "mainhand"), 0);
    // fortunate adds a flat 1 from modifier level 2 on (its modifier level condition), nothing at level 1.
    assert_eq!(armor_looting(&piece(&[("tconstruct:fortunate", 1)]), &env, "head"), 0);
    assert_eq!(armor_looting(&piece(&[("tconstruct:fortunate", 2)]), &env, "head"), 1);
    // luck I adds one in its slots; a piece without an armor looting modifier adds nothing.
    assert_eq!(armor_looting(&piece(&[("tconstruct:luck", 1)]), &env, "chest"), 1);
    assert_eq!(armor_looting(&piece(&[]), &env, "head"), 0);
}

/// The whole loaded original, as one world for end-to-end checks.
pub(crate) struct World { pub(crate) materials: Materials, pub(crate) registry: Registry, pub(crate) tools: Vec<(Identifier, Data)> }
pub(crate) fn world() -> World {
    let mut materials = Materials::load(&files("materials/definition"), &conditions());
    let mut problems = Vec::new();
    materials.stats = crate::material_stats::load(&stacks("materials/stats"), &mut problems);
    materials.traits = crate::material_traits::load(&stacks("materials/traits"), &mut problems);
    assert!(problems.is_empty(), "{problems:?}");
    materials.server_loaded = true;
    let mut registry = Registry::default();
    // the materials' traits that are Java classes in the original (copper's dwarven...), which `modifiers::register` adds before the first reload;
    // the tests that need overslime register its code modifiers themselves
    for modifier in crate::modifiers::unported_code_modifiers().unwrap() { registry.register_static(modifier).unwrap(); }
    registry.reload(&files("modifiers"), &conditions());
    let tools = files("tool_definitions").into_iter().map(|(id, value)| (id, Data::decode(&value).expect("shipped definition"))).collect();
    World { materials, registry, tools }
}

#[test]
fn a_pickaxe_of_shipped_wood_builds_repairs_and_breaks_through_the_station_dispatch() {
    use crate::tool_station::{lookup, Context, Input, Outcome};
    use crate::tool_materials::MaterialVariant;
    let world = world();
    let tools: Vec<(Identifier, &Data)> = world.tools.iter().filter(|(_, data)| !data.tool_parts().is_empty())
        .map(|(id, data)| (id.clone(), data)).collect();
    let item_max = |_: &Identifier| 1;
    let ctx = Context { materials: &world.materials, modifiers: &world.registry, item_tags_loaded: true, item_tags: None, tools: &tools, item_max: &item_max };
    let wood = MaterialVariant::parse("tconstruct:wood").unwrap();
    let part = |item: &str| Some(Input::Part { item: Identifier::parse(item).unwrap(), material: wood.clone() });
    let inputs = [part("tconstruct:pick_head"), part("tconstruct:tool_handle"), part("tconstruct:tool_binding"), None];
    let Outcome::Crafted(built) = lookup(&ctx, None, &inputs).unwrap() else { panic!("the shipped pickaxe did not build") };
    assert_eq!(built.result.item.to_string(), "tconstruct:pickaxe");
    let data = &built.result.custom_data;
    assert!(crate::tool_stack::durability(data) > 1, "wood parts give the tool real durability");
    assert!(crate::tool_stack::stat(data, crate::tool_stats::Stat::MiningSpeed) > 1.0);
    assert_eq!(crate::tool_stack::materials(data, true).as_slice().len(), 3);
    assert!(built.result.max_damage.is_some() && built.result.tool.is_some(), "the slot publishes its bar and mining rules");
    // The shipped definition's area module binds now: the pickaxe carries its box iterator.
    let (_, pickaxe) = tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap();
    assert!(pickaxe.aoe().is_some(), "the pickaxe definition binds its aoe_iterator");
    assert!(!pickaxe.unbound_modules().iter().any(|module| module.path() == "box_aoe"));
}

#[test]
fn the_model_strings_of_a_shipped_pickaxe_follow_its_material_slots_through_a_head_swap_and_a_part_carries_its_material() {
    use crate::tool_station::{lookup, Context, Input, Outcome};
    use crate::tool_materials::MaterialVariant;
    let world = world();
    let tools: Vec<(Identifier, &Data)> = world.tools.iter().filter(|(_, data)| !data.tool_parts().is_empty())
        .map(|(id, data)| (id.clone(), data)).collect();
    let item_max = |_: &Identifier| 1;
    let ctx = Context { materials: &world.materials, modifiers: &world.registry, item_tags_loaded: true, item_tags: None, tools: &tools, item_max: &item_max };
    let part = |item: &str, material: &str| Some(Input::Part { item: Identifier::parse(item).unwrap(), material: MaterialVariant::parse(material).unwrap() });
    let strings = |slot: &pluto_lib::ItemSlot| slot.custom_model_data.as_ref().map(|data| data.strings.clone());
    let inputs = [part("tconstruct:pick_head", "tconstruct:stone"), part("tconstruct:tool_handle", "tconstruct:wood"), part("tconstruct:tool_binding", "tconstruct:iron"), None];
    let Outcome::Crafted(built) = lookup(&ctx, None, &inputs).unwrap() else { panic!("the shipped pickaxe did not build") };
    assert_eq!(crate::tool_stack::materials(&built.result.custom_data, true).get(0).to_string(), "tconstruct:stone");
    assert_eq!(strings(&built.result), Some(vec!["stone".to_owned(), "wood".to_owned(), "iron".to_owned()]), "one sprite key per material slot, in slot order");
    // Swapping the head rewrites the first key and leaves the others.
    let swap = [part("tconstruct:pick_head", "tconstruct:iron"), None, None, None];
    let Outcome::Crafted(swapped) = lookup(&ctx, Some(&built.result), &swap).unwrap() else { panic!("the head did not swap") };
    assert_eq!(strings(&swapped.result), Some(vec!["iron".to_owned(), "wood".to_owned(), "iron".to_owned()]));
    // Wear and repair rebuild the visuals without disturbing the keys.
    let mut worn = swapped.result.clone();
    crate::tool_slot::damage_slot(&mut worn, &ctx.environment(&built.result.item, tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1), 3, true, true, None, &mut || 1.0).unwrap();
    assert_eq!(strings(&worn), strings(&swapped.result));
    // A part's only key is its material; an unknown material has none, and a stack with no materials has no data at all.
    let mut head = pluto_lib::ItemSlot::new(Identifier::parse("tconstruct:pick_head").unwrap(), 1);
    crate::tool_materials::set_part_material(&mut head, &MaterialVariant::parse("tconstruct:rose_gold").unwrap());
    assert_eq!(strings(&head), Some(vec!["rose_gold".to_owned()]));
    crate::tool_materials::set_part_material(&mut head, &MaterialVariant::unknown());
    assert_eq!(strings(&head), Some(vec![String::new()]));
    let plain = pluto_lib::ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
    assert_eq!(crate::tool_materials::model_strings(&plain.custom_data), None);
}

#[test]
fn shipped_stat_boost_modifiers_change_a_rebuilt_pickaxe() {
    use crate::{modifier_entries::List, tool_stack::{rebuild_stats, stat, Environment, TAG_UPGRADES, TAG_MATERIALS}, tool_stats::Stat};
    use pluto_lib::{Nbt, NbtCompound};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let build = |upgrades: Option<List>| {
        let mut nbt = NbtCompound::new();
        nbt.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        if let Some(list) = upgrades { nbt.insert(TAG_UPGRADES.into(), list.to_nbt()); }
        assert!(rebuild_stats(&mut nbt, &env).unwrap());
        nbt
    };
    let plain = build(None);
    let sharp = build(Some(List::default().with_modifier(Identifier::parse("tconstruct:sharpness").unwrap(), 3).unwrap()));
    // sharpness: +0.75 attack damage per level, three levels.
    let gained = stat(&sharp, Stat::AttackDamage) - stat(&plain, Stat::AttackDamage);
    assert!((gained - 2.25).abs() < 1e-4, "sharpness III added {gained}");
    // ductile: +10% durability per level (multiply_base), so durability grows.
    let ductile = build(Some(List::default().with_modifier(Identifier::parse("tconstruct:ductile").unwrap(), 2).unwrap()));
    assert!(crate::tool_stack::durability(&ductile) > crate::tool_stack::durability(&plain));
    // The modifiers themselves are recorded in the tool's modifier list.
    let Nbt::List(listed) = &sharp["tic_modifiers"] else { panic!() };
    assert!(listed.iter().any(|entry| matches!(entry, Nbt::Compound(c) if c.get("name") == Some(&Nbt::String("tconstruct:sharpness".into())))));
}

#[test]
fn every_shipped_disguise_binds_and_a_worn_piece_halves_the_range_per_level_for_its_mob() {
    use crate::{mob_disguise, modifier_entries::List, tool_stack::{mob_disguises, rebuild_stats, volatile_flag, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{Nbt, NbtCompound};
    let world = world();
    let disguises: Vec<Identifier> = files("modifiers").into_iter().map(|(id, _)| id).filter(|id| id.path().ends_with("_disguise")).collect();
    assert_eq!(disguises.len(), 14, "the original ships fourteen disguises");
    for id in &disguises {
        let modifier = world.registry.get(id).expect("the shipped disguise loads");
        assert!(modifier.unbound.is_empty(), "{id} has unported modules: {:?}", modifier.unbound);
        let found: Vec<_> = modifier.mob_disguise_modules().collect();
        assert_eq!(found.len(), 1, "{id} is one mob_disguise module");
        let entity = found[0].mob_disguise(&crate::modifier_entries::Entry::new(id.clone(), 1)).unwrap().0;
        assert_eq!(entity.to_string(), format!("minecraft:{}", id.path().trim_end_matches("_disguise")), "{id} disguises as its own mob");
    }
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let piece = |upgrades: &[(&str, i32)]| {
        let mut nbt = NbtCompound::new();
        nbt.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        let mut list = List::default();
        for (id, level) in upgrades { list = list.with_modifier(Identifier::parse(id).unwrap(), *level).unwrap(); }
        nbt.insert(TAG_UPGRADES.into(), list.to_nbt());
        assert!(rebuild_stats(&mut nbt, &env).unwrap());
        nbt
    };
    // Enderman disguise II and skeleton disguise I on one piece, a second skeleton disguise on another: counts add per mob type.
    let first = piece(&[("tconstruct:enderman_disguise", 2), ("tconstruct:skeleton_disguise", 1)]);
    let second = piece(&[("tconstruct:skeleton_disguise", 1)]);
    let counts = mob_disguise::count(mob_disguises(&first, &env).into_iter().chain(mob_disguises(&second, &env)));
    assert_eq!(mob_disguise::visibility_factor(&counts, "minecraft:enderman"), 0.25);
    assert_eq!(mob_disguise::visibility_factor(&counts, "minecraft:skeleton"), 0.25);
    assert_eq!(mob_disguise::visibility_factor(&counts, "minecraft:zombie"), 1.0);
    assert!(volatile_flag(&first, crate::mob_disguise_hook::ENDERMASK), "the enderman disguise makes the piece an ender mask");
    assert!(!volatile_flag(&second, crate::mob_disguise_hook::ENDERMASK));
    assert!(mob_disguises(&piece(&[]), &env).is_empty());
}

#[test]
fn shipped_worn_helmet_given_without_stats_is_built_by_its_first_wear_and_keeps_the_ender_mask_flag() {
    // The smoke scenario MOB DISGUISE: a plate helmet of [cobalt, iron] handed over with no tic_stats, then worn down once.
    use crate::{modifier_entries::List, tool_slot, tool_stack::{is_broken, is_initialized, volatile_flag, Environment, TAG_MATERIALS, TAG_MODIFIERS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let helmet = &world.tools.iter().find(|(id, _)| id.path() == "plate_helmet").unwrap().1;
    let helmet_id = Identifier::parse("tconstruct:plate_helmet").unwrap();
    let armor_tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:plate_helmet"], &[("tconstruct:modifiable/armor", &["tconstruct:plate_helmet"])]).unwrap();
    let env = Environment { definition: Some(helmet), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&helmet_id), item_tags: Some(&armor_tags) };
    let disguise = Identifier::parse("tconstruct:enderman_disguise").unwrap();
    let given = |key: &str| {
        let mut slot = ItemSlot::new(helmet_id.clone(), 1);
        slot.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:cobalt", "tconstruct:iron"].map(|m| Nbt::String(m.into())).to_vec()));
        slot.custom_data.insert(key.into(), List::default().with_modifier(disguise.clone(), 1).unwrap().to_nbt());
        assert!(!is_initialized(&slot.custom_data));
        slot
    };
    // Saved as an upgrade (the input of a rebuild), the first wear builds the piece, which is not broken and carries the flag.
    let mut worn = given(TAG_UPGRADES);
    let hit = tool_slot::damage_slot(&mut worn, &env, 1, true, true, None, &mut || 1.0).unwrap();
    assert!(is_initialized(&worn.custom_data) && !is_broken(&worn.custom_data), "{:?}", worn.custom_data);
    assert_eq!((hit.applied, hit.broke), (1, false));
    assert!(worn.max_damage.is_some_and(|max| max > 2), "a real durability, not the 1-point default: {:?}", worn.max_damage);
    assert!(volatile_flag(&worn.custom_data, crate::mob_disguise_hook::ENDERMASK), "{:?}", worn.custom_data);
    // tic_modifiers is a rebuild's output, never its input: a piece given only that list loses it, as in the original.
    let mut output_only = given(TAG_MODIFIERS);
    tool_slot::damage_slot(&mut output_only, &env, 1, true, true, None, &mut || 1.0).unwrap();
    assert!(!is_broken(&output_only.custom_data) && !volatile_flag(&output_only.custom_data, crate::mob_disguise_hook::ENDERMASK));
}

#[test]
fn shipped_reinforced_and_cultivated_change_damage_and_repair_of_a_real_pickaxe() {
    use crate::{modifier_entries::List, tool_slot, tool_stack::{current_durability, rebuild_stats, repair_factor, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let tool = |upgrades: &[(&str, i32)]| {
        let mut slot = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
        slot.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        let mut list = List::default();
        for (id, level) in upgrades { list = list.with_modifier(Identifier::parse(id).unwrap(), *level).unwrap(); }
        slot.custom_data.insert(TAG_UPGRADES.into(), list.to_nbt());
        assert!(rebuild_stats(&mut slot.custom_data, &env).unwrap());
        slot
    };
    // Reinforced V cancels 0.025 * 5 * 6 = 75% of damage: a roll below it cancels, above it lands.
    let mut reinforced = tool(&[("tconstruct:reinforced", 5)]);
    let hit = tool_slot::damage_slot(&mut reinforced, &env, 8, true, true, None, &mut || 0.5).unwrap();
    assert_eq!(hit.applied, 0, "every roll below 75% cancels its point");
    let hit = tool_slot::damage_slot(&mut reinforced, &env, 8, true, true, None, &mut || 0.9).unwrap();
    assert_eq!(hit.applied, 8, "every roll above 75% lets its point through");
    // From level 6 the flat branch counts: 0.5 + 0.05 * 10 = 100% at level X, so even the highest roll cancels the point.
    let mut ten = tool(&[("tconstruct:reinforced", 10)]);
    let hit = tool_slot::damage_slot(&mut ten, &env, 8, true, true, None, &mut || 0.99).unwrap();
    assert_eq!(hit.applied, 0, "ten levels of reinforced cancel every point");
    // An unmodified pickaxe takes the full damage regardless of the roll.
    let mut plain = tool(&[]);
    assert_eq!(tool_slot::damage_slot(&mut plain, &env, 5, true, true, None, &mut || 0.0).unwrap().applied, 5);
    // Cultivated scales repair by factor * (1 + 0.5 per level). Wood parts already bring it at level 3
    // (three wood parts), so a plain wood pickaxe repairs at x2.5 and two upgrade levels reach x3.5.
    let cultivated = tool(&[("tconstruct:cultivated", 2)]);
    assert!((repair_factor(&plain.custom_data, &env, 10.0) - 25.0).abs() < 1e-4);
    assert!((repair_factor(&cultivated.custom_data, &env, 10.0) - 35.0).abs() < 1e-4);
    let _ = current_durability(&plain.custom_data);
}

pub(crate) fn recursive_files(folder: &str, prefix: &str) -> Vec<(Identifier, Value)> {
    fn walk(dir: &std::path::Path, relative: String, out: &mut Vec<(Identifier, Value)>) {
        for entry in std::fs::read_dir(dir).expect("folder") {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_str().unwrap().to_owned();
            if path.is_dir() { walk(&path, format!("{relative}/{name}"), out); continue; }
            if let Some(stem) = name.strip_suffix(".json") {
                let value = serde_json::from_slice(&std::fs::read(&path).unwrap()).expect("valid JSON");
                out.push((Identifier::new("tconstruct", &format!("{relative}/{stem}")).unwrap(), value));
            }
        }
    }
    let mut found = Vec::new();
    walk(&root().join("../recipes").join(folder), prefix.to_owned(), &mut found);
    found
}

#[test]
fn shipped_material_recipes_load_without_problems_and_horns_match_by_instrument() {
    let files = recursive_files("tools/materials", "tools/materials");
    assert_eq!(files.len(), 217, "the 176 that shipped before plus the sixteen integration materials' 41 (their own conditions keep them off while the tags are empty)");
    let recipes = crate::material_recipes::Recipes::load(&files, &conditions());
    assert!(recipes.problems.is_empty(), "{:#?}", recipes.problems);
    assert!(recipes.len() > 60, "{} loaded", recipes.len());
    // Tag-gated recipes (planks, logs) stay off while the item tags are empty, as their conditions say;
    // item-keyed recipes resolve: the mod's own blazewood and vanilla flint.
    let blazewood = recipes.find(&Identifier::parse("tconstruct:blazewood").unwrap(), &|_, _| false).expect("blazewood");
    assert_eq!(blazewood.material.to_string(), "tconstruct:blazewood");
    let flint = recipes.find(&Identifier::parse("minecraft:flint").unwrap(), &|_, _| false).expect("flint");
    assert_eq!(flint.material.to_string(), "tconstruct:flint");
    // The goat horn variants answer only a horn carrying their instrument; the default horn recipe's
    // condition (every goat horn is a variant horn) keeps it out, so a plain horn matches nothing.
    let horn = Identifier::parse("minecraft:goat_horn").unwrap();
    let none = |_: &Identifier, _: &Identifier| false;
    assert!(recipes.find(&horn, &none).is_none());
    for (instrument, material) in [("minecraft:sing_goat_horn", "tconstruct:horn#minecraft.sing_goat_horn"), ("minecraft:ponder_goat_horn", "tconstruct:horn#minecraft.ponder_goat_horn")] {
        let instrument = Identifier::parse(instrument).unwrap();
        let facts = crate::material_recipes::StackFacts { instrument: Some(&instrument), ..Default::default() };
        assert_eq!(recipes.find_facts(&horn, &none, &facts).expect("the horn variant").material.to_string(), material);
    }
}

#[test]
fn shipped_modifier_recipes_load_and_only_unported_ingredient_types_are_left_out() {
    let files = recursive_files("tools/modifiers", "tools/modifiers");
    assert_eq!(files.len(), 367, "116 modifier, 45 incremental, 38 swappable, 135 salvage, 2 removal, 5 multilevel, 16 overslime and 10 enchantment converting records");
    let recipes = crate::modifier_recipes::Recipes::load(&files, &conditions());
    for problem in &recipes.problems {
        assert!(problem.contains("unimplemented ingredient type") || problem.contains("unimplemented tool predicate"), "{problem}");
    }
    assert!(recipes.len() > 160, "{} loaded, {:#?}", recipes.len(), recipes.problems);
}

#[test]
fn the_shipped_reinforced_recipe_fills_a_level_in_two_partial_applications() {
    use crate::{modifier_entries::List, tool_stack::{rebuild_stats, Environment, TAG_MATERIALS, TAG_PERSISTENT, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt, NbtCompound};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let recipes = crate::modifier_recipes::Recipes::load(&recursive_files("tools/modifiers", "tools/modifiers"), &conditions());
    let in_tag = |item: &Identifier, tag: &Identifier| item.path() == "pickaxe" && tag.path() == "modifiable/durability";
    let reinforced = Identifier::parse("tconstruct:reinforced").unwrap();
    let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
    tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
    let mut persistent = NbtCompound::new();
    persistent.insert("upgrades".into(), Nbt::Int(3));
    tool.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
    assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
    let reinforcement = |count: u32| [Some((Identifier::parse("tconstruct:emerald_reinforcement").unwrap(), count))];
    let upgrades = |tool: &ItemSlot| List::from_nbt(tool.custom_data.get(TAG_UPGRADES));
    let free = |tool: &ItemSlot| match tool.custom_data.get(TAG_PERSISTENT) { Some(Nbt::Compound(data)) => crate::modifier_entries::nbt_int(data.get("upgrades")), _ => 0 };

    let first = recipes.find(&tool.item, &reinforcement(2), &in_tag).expect("the shipped recipe matches a pickaxe");
    let half = first.apply(&tool, &env, &reinforcement(2), &in_tag).unwrap().expect("two of four items apply");
    assert_eq!((upgrades(&half).entry(&reinforced).amount(0), upgrades(&half).entry(&reinforced).needed(), free(&half)), (2, 4, 2), "a partial level spends one slot");
    assert_eq!(first.consumption(&tool, &half, &reinforcement(2), &in_tag).0, vec![2]);

    // Continuing the partial level spends no further slot, and two more items finish it.
    let full = first.apply(&half, &env, &reinforcement(2), &in_tag).unwrap().expect("the second half applies");
    assert_eq!((upgrades(&full).level(&reinforced), upgrades(&full).entry(&reinforced).needed(), free(&full)), (1, 0, 2));
    assert_eq!(first.consumption(&half, &full, &reinforcement(5), &in_tag).0, vec![2], "only what the level still needed is taken");
    // reinforced reduces damage taken (see the damage test); it does not change the durability stat.
}

#[test]
fn shipped_diamond_and_emerald_raise_the_harvest_tier_of_a_wood_pickaxe() {
    use crate::{modifier_entries::List, tool_stack::{rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{Nbt, NbtCompound};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let tier_with = |modifier: Option<&str>| {
        let mut nbt = NbtCompound::new();
        nbt.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        if let Some(id) = modifier { nbt.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse(id).unwrap(), 1).unwrap().to_nbt()); }
        assert!(rebuild_stats(&mut nbt, &env).unwrap());
        let Some(Nbt::Compound(stats)) = nbt.get(crate::tool_stack::TAG_STATS) else { panic!("no stats") };
        match stats.get("tconstruct:harvest_tier") { Some(Nbt::String(tier)) => tier.clone(), other => panic!("no tier: {other:?}") }
    };
    assert_eq!(tier_with(None), "minecraft:wood");
    assert_eq!(tier_with(Some("tconstruct:emerald")), "minecraft:iron");
    assert_eq!(tier_with(Some("tconstruct:diamond")), "minecraft:diamond");
}

#[test]
fn the_shipped_part_builder_makes_wood_parts_from_logs_and_hands_back_planks() {
    use crate::part_builder_recipes::{Context, Inventory, Recipes};
    let world = world();
    let part_files = recursive_files("tools/parts/builder", "tools/parts/builder");
    assert_eq!(part_files.len(), 24);
    let recipes = Recipes::load(&part_files, &conditions());
    assert!(recipes.problems.is_empty(), "{:#?}", recipes.problems);
    assert_eq!(recipes.len(), 24);
    let materials = crate::material_recipes::Recipes::load(&recursive_files("tools/materials", "tools/materials"), &conditions());
    let parts = crate::tool_parts::registry();
    let in_tag = |item: &Identifier, tag: &Identifier| match tag.path() {
        "patterns/default" => item.path() == "pattern",
        "oak_logs" => item.path() == "oak_log",
        _ => false,
    };
    let ctx = Context { materials: &world.materials, parts: &parts, in_tag: &in_tag };
    let pattern = Identifier::parse("tconstruct:pattern").unwrap();
    let planks = Identifier::parse("minecraft:oak_log").unwrap();
    let wood = materials.find(&planks, &in_tag).expect("logs are a material");
    assert_eq!(wood.material.to_string(), "tconstruct:wood");
    let inv = |count: u32| Inventory { pattern_item: Some(&pattern), stack: Some((&planks, count)), material: Some(wood) };

    // With planks in the slot, every wood-capable part is offered; a pattern with nothing placed offers all 24.
    let offered = recipes.patterns(&inv(8), &ctx);
    assert!(offered.iter().any(|p| p.path() == "pick_head") && offered.iter().any(|p| p.path() == "tool_handle"), "{offered:?}");
    assert_eq!(recipes.patterns(&Inventory { pattern_item: Some(&pattern), stack: None, material: None }, &ctx).len(), 24);

    let head = Identifier::parse("tconstruct:pick_head").unwrap();
    // A log is worth 4 and a pick head costs 2: one log pays for it and half is handed back as 2 planks.
    assert!(recipes.find(&head, &inv(0), &ctx).is_none(), "no logs, no part");
    let recipe = recipes.find(&head, &inv(1), &ctx).expect("one log makes a pick head");
    assert_eq!(wood.leftover_for(recipe.cost).map(|(source, count)| (source.clone(), count)),
        Some((crate::material_recipes::LeftoverSource::Item(Identifier::parse("minecraft:oak_planks").unwrap()), 2)));
    let crafted = recipe.assemble(&inv(1));
    assert_eq!((crafted.item.to_string(), crate::tool_materials::part_material(&crafted.custom_data).to_string()), ("tconstruct:pick_head".to_owned(), "tconstruct:wood".to_owned()));
    assert_eq!((recipe.items_used(&inv(1)), crafted.count), (1, 1), "a leftover recipe does not multiply the parts");
}

#[test]
fn the_shipped_salvage_returns_the_slot_when_a_sharpness_level_is_removed() {
    use crate::{modifier_entries::List, tool_stack::{rebuild_stats, Environment, TAG_MATERIALS, TAG_PERSISTENT, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt, NbtCompound};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let salvages = crate::modifier_removal::Salvages::load(&recursive_files("tools/modifiers", "tools/modifiers"), &conditions());
    assert_eq!(salvages.len(), 133, "135 records, two behind data conditions that are false here: {:?}", salvages.problems);
    let in_tag = |_: &Identifier, tag: &Identifier| tag.path() == "modifiable/melee";
    let sharp = Identifier::parse("tconstruct:sharpness").unwrap();
    let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
    tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
    tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(sharp.clone(), 2).unwrap().to_nbt());
    let mut persistent = NbtCompound::new();
    persistent.insert("upgrades".into(), Nbt::Int(1));
    tool.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
    assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
    let damage = |tool: &ItemSlot| crate::tool_stack::stat(&tool.custom_data, crate::tool_stats::Stat::AttackDamage);

    let once = crate::modifier_removal::remove(&tool, &env, &sharp, &salvages, &in_tag).unwrap().expect("a level comes off");
    assert_eq!(List::from_nbt(once.custom_data.get(TAG_UPGRADES)).level(&sharp), 1);
    let Some(Nbt::Compound(slots)) = once.custom_data.get(TAG_PERSISTENT) else { panic!("persistent data") };
    assert_eq!(crate::modifier_entries::nbt_int(slots.get("upgrades")), 2, "the salvaged slot is handed back");
    assert!(damage(&once) < damage(&tool), "one level less sharpness");
    let twice = crate::modifier_removal::remove(&once, &env, &sharp, &salvages, &in_tag).unwrap().expect("the last level comes off");
    assert_eq!(List::from_nbt(twice.custom_data.get(TAG_UPGRADES)).level(&sharp), 0);
    assert!(crate::modifier_removal::remove(&twice, &env, &sharp, &salvages, &in_tag).unwrap().is_err(), "nothing left to remove");
}

#[test]
fn the_shipped_sponge_and_venom_removals_load_and_the_venom_one_matches_a_filled_container() {
    let removals = crate::worktable_recipes::Removals::load(&recursive_files("tools/modifiers", "tools/modifiers"), &conditions());
    assert_eq!(removals.len(), 2, "{:?}", removals.problems);
    assert!(removals.problems.is_empty(), "{:?}", removals.problems);
    // The venom removal takes a container holding venom: a bucket of it (1000) or a bottle (250).
    let tags = crate::fluid_ingredient::FluidTags::load(&[(Identifier::parse("tconstruct:venom").unwrap(), serde_json::json!({"values": ["tconstruct:venom", "tconstruct:flowing_venom"]}))]);
    let venom = Identifier::parse("tconstruct:venom").unwrap();
    let in_tag = |item: &Identifier, tag: &Identifier| tag.to_string() == "tconstruct:modifiable" && item.to_string() == "tconstruct:pickaxe";
    let tool = (Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
    let facts = |amount: i32| [crate::material_recipes::StackFacts { fluid: Some((&venom, amount)), fluid_tags: Some(&tags), ..Default::default() }];
    let bucket = [Some((Identifier::parse("tconstruct:venom_bucket").unwrap(), 1))];
    let bottle = [Some((Identifier::parse("tconstruct:venom_bottle").unwrap(), 1))];
    assert!(removals.find_facts(&tool, &bucket, &facts(1000), &in_tag).is_some_and(|recipe| recipe.id.to_string().contains("remove_modifier_venom")), "a bucket of venom");
    assert!(removals.find_facts(&tool, &bottle, &facts(250), &in_tag).is_some(), "a bottle of venom is enough");
    assert!(removals.find_facts(&tool, &bucket, &facts(100), &in_tag).is_none(), "100 of venom is neither a bucket nor a bottle");
    assert!(removals.find(&tool, &bucket, &in_tag).is_none(), "a container whose contents are unknown matches nothing");
}

#[test]
fn slime_balls_fill_overslime_on_a_pickaxe_and_the_shield_soaks_damage_first() {
    use crate::{overslime, tool_stack::{rebuild_stats, Environment, TAG_MATERIALS}};
    use pluto_lib::{ItemSlot, Nbt};
    let mut world = world();
    for modifier in overslime::code_modifiers().unwrap() { world.registry.register_static(modifier).unwrap(); }
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let recipes = overslime::Recipes::load(&recursive_files("tools/modifiers", "tools/modifiers"), &conditions());
    assert_eq!(recipes.len(), 16, "{:?}", recipes.problems);
    let in_tag = |item: &Identifier, tag: &Identifier| item.path() == "pickaxe" && tag.path() == "modifiable/durability";
    let slime = Identifier::parse("minecraft:slime_ball").unwrap();
    let inputs = [Some((slime.clone(), 2)), None];
    let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
    tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
    assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());

    let recipe = recipes.find(&tool.item, &inputs, &in_tag).expect("a slime ball recipe matches");
    let slimy = recipe.apply(&tool, &env, &inputs, &in_tag).unwrap().expect("overslime applies");
    let capacity = overslime::capacity(&slimy.custom_data);
    assert!(capacity > 0, "the modifier brings capacity");
    assert_eq!(overslime::amount(&slimy.custom_data), (2 * 20).min(capacity), "two slime balls restore 20 each, capped");
    // The stat penalties need the item tags (a tool with no item context skips them), so they are not checked here.
    let used = recipe.consumption(&tool, &slimy, &env, &inputs, &in_tag);
    assert_eq!(used, vec![(overslime::amount(&slimy.custom_data) as u32).div_ceil(20), 0]);

    // The shield soaks damage first; what it cannot cover goes on to the tool.
    let mut worn = slimy.clone();
    let shield = overslime::amount(&worn.custom_data);
    assert_eq!(overslime::absorb(&mut worn.custom_data, &env, None, shield - 1), 0);
    assert_eq!(overslime::amount(&worn.custom_data), 1);
    assert_eq!(overslime::absorb(&mut worn.custom_data, &env, None, 5), 4);
    assert_eq!(overslime::amount(&worn.custom_data), 0);
    // Each overworked level doubles what a restore gives (1 + level).
    let mut boosted = tool.clone();
    boosted.custom_data.insert(crate::tool_stack::TAG_UPGRADES.into(), crate::modifier_entries::List::default()
        .with_modifier(overslime::id(), 1).unwrap().with_modifier(Identifier::parse("tconstruct:overworked").unwrap(), 2).unwrap().to_nbt());
    assert!(rebuild_stats(&mut boosted.custom_data, &env).unwrap());
    assert_eq!(overslime::overworked_bonus(&boosted.custom_data, &env), 3);
    overslime::add_amount(&mut boosted.custom_data, &env, 4);
    assert_eq!(overslime::amount(&boosted.custom_data), 12.min(overslime::capacity(&boosted.custom_data)));
    // Full overslime cannot take more slime.
    let mut full = slimy;
    overslime::add_amount(&mut full.custom_data, &env, 10_000);
    assert!(recipe.apply(&full, &env, &inputs, &in_tag).unwrap().is_err());
}

#[test]
fn creative_slot_adds_the_slots_stored_in_the_creative_compound_and_forgets_them_on_removal() {
    use crate::{overslime, tool_stack::{on_removed, rebuild_stats, Environment, TAG_MATERIALS, TAG_PERSISTENT, TAG_UPGRADES, TAG_VOLATILE}};
    use pluto_lib::{ItemSlot, Nbt, NbtCompound};
    let mut world = world();
    for modifier in overslime::code_modifiers().unwrap() { world.registry.register_static(modifier).unwrap(); }
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let slots = |tool: &ItemSlot| match tool.custom_data.get(TAG_VOLATILE) {
        Some(Nbt::Compound(data)) => crate::modifier_entries::nbt_int(data.get("upgrades")),
        _ => 0,
    };
    let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
    tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
    assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
    let before = slots(&tool);
    let mut stored = NbtCompound::new();
    stored.insert("upgrades".into(), Nbt::Int(3));
    let mut persistent = NbtCompound::new();
    persistent.insert("tconstruct:creative".into(), Nbt::Compound(stored));
    tool.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
    tool.custom_data.insert(TAG_UPGRADES.into(), crate::modifier_entries::List::default()
        .with_modifier(Identifier::parse("tconstruct:creative_slot").unwrap(), 1).unwrap().to_nbt());
    assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
    assert_eq!(slots(&tool), before + 3, "the stored slots join the volatile data");
    // Removing the modifier forgets the stored compound.
    let original = tool.custom_data.clone();
    tool.custom_data.insert(TAG_UPGRADES.into(), crate::modifier_entries::List::default().to_nbt());
    assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
    on_removed(&original, &mut tool.custom_data, &env);
    match tool.custom_data.get(TAG_PERSISTENT) { Some(Nbt::Compound(data)) => assert!(!data.contains_key("tconstruct:creative")), _ => {} }
    assert_eq!(slots(&tool), before);
}

#[test]
fn stat_override_adds_stored_stats_dropping_the_base_and_multiplies_stored_factors() {
    use crate::{modifier_entries::List, overslime, tool_stack::{rebuild_stats, stat, Environment, TAG_MATERIALS, TAG_PERSISTENT, TAG_UPGRADES}, tool_stats::Stat};
    use pluto_lib::{Nbt, NbtCompound};
    let mut world = world();
    for modifier in overslime::code_modifiers().unwrap() { world.registry.register_static(modifier).unwrap(); }
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let build = |bonus: Option<f32>, factor: Option<f32>| {
        let mut nbt = NbtCompound::new();
        nbt.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        let mut persistent = NbtCompound::new();
        for (key, value) in [("tconstruct:override_bonus", bonus), ("tconstruct:override_multiplier", factor)] {
            if let Some(value) = value {
                let mut stats = NbtCompound::new();
                stats.insert("tconstruct:attack_damage".into(), Nbt::Float(value));
                persistent.insert(key.into(), Nbt::Compound(stats));
            }
        }
        nbt.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
        nbt.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse("tconstruct:stat_override").unwrap(), 1).unwrap().to_nbt());
        assert!(rebuild_stats(&mut nbt, &env).unwrap());
        nbt
    };
    // update() sums the value and removes the implicit base, so the stat becomes the stored number plus other additions.
    let plain_damage = stat(&build(None, None), Stat::AttackDamage);
    let overridden = stat(&build(Some(7.5), None), Stat::AttackDamage);
    assert!((overridden - (plain_damage + 7.5)).abs() < 1e-3, "update sums with the definition stats: {overridden} vs {plain_damage}");
    let plain = stat(&build(None, None), Stat::AttackDamage);
    assert!((stat(&build(None, Some(2.0)), Stat::AttackDamage) - plain * 2.0).abs() < 1e-3);
}

#[test]
fn a_damaged_wood_pickaxe_is_repaired_with_wood_material_items() {
    use crate::{tool_station::{lookup, Context, Input, Outcome}, tool_stack::{rebuild_stats, Environment, TAG_MATERIALS, TAG_DAMAGE}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let tools: Vec<(Identifier, &Data)> = world.tools.iter().filter(|(_, data)| !data.tool_parts().is_empty()).map(|(id, data)| (id.clone(), data)).collect();
    let item_max = |_: &Identifier| 1;
    let ctx = Context { materials: &world.materials, modifiers: &world.registry, item_tags_loaded: true, item_tags: None, tools: &tools, item_max: &item_max };
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
    tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
    assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
    tool.custom_data.insert(TAG_DAMAGE.into(), Nbt::Int(20));
    let planks = Input::Material { item: Identifier::parse("minecraft:oak_planks").unwrap(), material: Identifier::parse("tconstruct:wood").unwrap(), value: 1, needed: 1, count: 8 };
    let outcome = lookup(&ctx, Some(&tool), &[Some(planks), None, None, None]).unwrap();
    match outcome {
        Outcome::Crafted(done) => assert!(crate::tool_stack::damage(&done.result.custom_data) < 20, "repair lowers the damage"),
        Outcome::Failure(error) => panic!("repair failed: {error}"),
        Outcome::NoMatch => panic!("no match for planks repairing a wood pickaxe"),
        _ => panic!("unexpected outcome"),
    }
}

#[test]
fn shipped_mining_speed_modifiers_change_the_dig_speed_of_a_rebuilt_pickaxe() {
    use crate::{mining_speed::Context, modifier_entries::List, tool_stack::{break_speed, rebuild_stats, Environment, TAG_DAMAGE, TAG_MATERIALS, TAG_PERSISTENT, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt, NbtCompound};
    let mut world = world();
    for modifier in crate::overslime::code_modifiers().unwrap() { world.registry.register_static(modifier).unwrap(); }
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let stone = Identifier::parse("minecraft:stone").unwrap();
    let melts = |tag: &Identifier| tag.to_string() == "tconstruct:can_melt";
    let dig = |nbt: &pluto_lib::NbtCompound, effective: bool, on_ground: bool| {
        let context = Context { block: &stone, block_in_tag: &melts, original: 4.0, multiplier: 1.0, effective, on_ground: Some(on_ground), eyes_in_water: Some(false), facts: Default::default() };
        break_speed(nbt, &env, &context, 4.0)
    };
    let build = |upgrades: Vec<(&str, i32)>, persistent: Option<(&str, i32)>, damage: i32| {
        let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        let mut list = List::default();
        for (name, level) in upgrades { list = list.with_modifier(Identifier::parse(name).unwrap(), level).unwrap(); }
        tool.custom_data.insert(TAG_UPGRADES.into(), list.to_nbt());
        if let Some((key, value)) = persistent {
            let mut data = NbtCompound::new();
            data.insert(key.into(), Nbt::Int(value));
            tool.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(data));
        }
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        tool.custom_data.insert(TAG_DAMAGE.into(), Nbt::Int(damage));
        tool.custom_data
    };
    let plain = build(vec![], None, 0);
    assert_eq!(dig(&plain, true, true), 4.0, "no modifier leaves the speed alone");
    // searing: +6 per level on blocks that can melt, only on effective blocks.
    let searing = build(vec![("tconstruct:searing", 2)], None, 0);
    assert!(dig(&searing, true, true) > 4.0 + 11.0, "searing II adds 12 times the tool multiplier: {}", dig(&searing, true, true));
    assert_eq!(dig(&searing, false, true), 4.0, "an ineffective block gets nothing");
    // airborne: +400% while off the ground, regardless of effectiveness.
    let airborne = build(vec![("tconstruct:airborne", 1)], None, 0);
    assert_eq!(dig(&airborne, true, true), 4.0);
    assert!(dig(&airborne, false, false) > 4.0, "airborne boosts in the air");
    // stonebound: the more durability lost, the faster (a percent formula over the tool's lost durability).
    let fresh = build(vec![("tconstruct:stonebound", 2)], None, 0);
    let worn = build(vec![("tconstruct:stonebound", 2)], None, 30);
    assert_eq!(dig(&fresh, true, true), 4.0);
    assert!(dig(&worn, true, true) > 4.0, "a worn stonebound tool digs faster: {}", dig(&worn, true, true));
    // overwield: overslime from persistent data speeds up broad tools; the pickaxe is not broad, so check it reads data.
    let overwield = build(vec![("tconstruct:overwield", 1)], Some(("tconstruct:overslime", 100)), 0);
    let _ = dig(&overwield, true, true);
}

#[test]
fn shipped_jagged_and_stonebound_change_melee_damage_with_the_tools_lost_durability() {
    use crate::{modifier_entries::List, tool_stack::{melee_damage, rebuild_stats, Environment, TAG_DAMAGE, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let hit = |modifier: &str, damage: i32| {
        let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse(modifier).unwrap(), 3).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        tool.custom_data.insert(TAG_DAMAGE.into(), Nbt::Int(damage));
        melee_damage(&tool.custom_data, &env, 2.0, 10.0, &|_| false, &Default::default())
    };
    assert_eq!(hit("tconstruct:jagged", 0), 10.0, "a fresh jagged tool changes nothing");
    assert!(hit("tconstruct:jagged", 40) > 10.0, "a worn jagged tool hits harder: {}", hit("tconstruct:jagged", 40));
    assert!(hit("tconstruct:stonebound", 40) < 10.0, "a worn stonebound tool hits softer: {}", hit("tconstruct:stonebound", 40));
}

#[test]
fn shipped_smite_and_bane_of_sssss_add_damage_only_to_the_right_targets() {
    use crate::{modifier_entries::List, tool_stack::{melee_damage, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let hit = |modifier: &str, tags: &'static [&'static str]| {
        let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse(modifier).unwrap(), 2).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        melee_damage(&tool.custom_data, &env, 2.0, 10.0, &|tag: &Identifier| tags.contains(&tag.to_string().as_str()), &Default::default())
    };
    assert!(hit("tconstruct:smite", &["minecraft:undead"]) > 10.0, "smite boosts undead targets");
    assert_eq!(hit("tconstruct:smite", &["minecraft:arthropod"]), 10.0, "smite leaves other targets alone");
    assert!(hit("tconstruct:bane_of_sssss", &["minecraft:arthropod"]) > 10.0, "bane of sssss boosts arthropods");
    assert_eq!(hit("tconstruct:bane_of_sssss", &[]), 10.0);
}

#[test]
fn shipped_speedy_publishes_a_movement_speed_attribute_for_its_slots() {
    use crate::{modifier_entries::List, tool_stack::{attribute_entries, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
    tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
    tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse("tconstruct:speedy").unwrap(), 3).unwrap().to_nbt());
    assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
    let held = attribute_entries(&tool.custom_data, &env, &["mainhand"]);
    assert_eq!(held.len(), 1, "{held:?}");
    assert_eq!(held[0]["type"], Nbt::String("minecraft:movement_speed".into()));
    assert_eq!(held[0]["operation"], Nbt::String("add_multiplied_total".into()));
    match held[0]["amount"] { Nbt::Double(amount) => assert!((amount - 0.3).abs() < 1e-5, "{amount}"), _ => panic!("amount is a double") }
    assert_eq!(attribute_entries(&tool.custom_data, &env, &["head"]).len(), 1, "speedy lists the head slot");
    assert!(attribute_entries(&tool.custom_data, &env, &["offhand"]).is_empty(), "speedy does not list the offhand");
}

fn damage_tag_stacks() -> Vec<JsonResourceStack> {
    fn walk(dir: &std::path::Path, relative: &str, out: &mut Vec<JsonResourceStack>) {
        for entry in std::fs::read_dir(dir).expect("tag folder") {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_str().unwrap().to_owned();
            if path.is_dir() { walk(&path, &format!("{relative}{name}/"), out); continue; }
            if let Some(stem) = name.strip_suffix(".json") {
                let value = serde_json::from_slice(&std::fs::read(&path).unwrap()).expect("valid JSON");
                out.push(JsonResourceStack { id: Identifier::new("tconstruct", &format!("{relative}{stem}")).unwrap(), layers: vec![Ok(value)] });
            }
        }
    }
    let mut stacks = Vec::new();
    walk(&root().join("tags/damage_types"), "", &mut stacks);
    stacks
}

#[test]
fn shipped_damage_tags_resolve_protection_tags_through_the_vanilla_ones() {
    use crate::damage_tags::DamageTags;
    let stacks = damage_tag_stacks();
    let (tags, problems) = DamageTags::load(&stacks);
    assert!(problems.is_empty(), "{problems:?}");
    let id = |text: &str| Identifier::parse(text).unwrap();
    assert!(tags.contains(&id("tconstruct:protection/fire"), &id("minecraft:lava")), "fire protection takes vanilla's fire tag");
    assert!(tags.contains(&id("tconstruct:protection/fire"), &id("minecraft:lightning_bolt")), "and lightning");
    assert!(tags.contains(&id("tconstruct:protection/blast"), &id("minecraft:explosion")));
    assert!(tags.contains(&id("tconstruct:protection/melee"), &id("minecraft:mob_attack")), "melee comes through is_melee");
    assert!(tags.contains(&id("tconstruct:protection/projectile"), &id("minecraft:arrow")));
    assert!(tags.contains(&id("tconstruct:protection/fall"), &id("minecraft:fall")));
    assert!(!tags.contains(&id("tconstruct:protection/fire"), &id("minecraft:arrow")), "an arrow is no fire damage");
}

#[test]
fn shipped_protection_modifiers_add_protection_against_the_sources_they_name() {
    use crate::{damage_tags::DamageTags, modifier_entries::List, protection::{damage_after_protection, DamageSource}, tool_stack::{protection, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let helmet = &world.tools.iter().find(|(id, _)| id.path() == "plate_helmet").unwrap().1;
    let helmet_id = Identifier::parse("tconstruct:plate_helmet").unwrap();
    let armor_tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:plate_helmet"], &[("tconstruct:modifiable/armor", &["tconstruct:plate_helmet"])]).unwrap();
    let env = Environment { definition: Some(helmet), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&helmet_id), item_tags: Some(&armor_tags) };
    let (tags, problems) = DamageTags::load(&damage_tag_stacks());
    assert!(problems.is_empty(), "{problems:?}");
    let id = |text: &str| Identifier::parse(text).unwrap();
    let protection_of = |modifier: &str, level: i32, damage_type: &str, indirect: bool| {
        let mut tool = ItemSlot::new(id("tconstruct:plate_helmet"), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(id(modifier), level).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        let damage_type = id(damage_type);
        protection(&tool.custom_data, &env, &DamageSource { damage_type: &damage_type, indirect, tags: &tags, attacker_in_tag: &|_| false, entities: Default::default() }, 0.0)
    };
    assert_eq!(protection_of("tconstruct:protection", 2, "minecraft:mob_attack", false), 2.5, "protection gives 1.25 a level");
    assert_eq!(protection_of("tconstruct:protection", 2, "minecraft:out_of_world", false), 0.0, "void damage bypasses protection");
    // The running value starts at the game's enchantment points in the hook; from 0 the enchantment's 2 fire points come off its own 3.
    assert_eq!(protection_of("tconstruct:fire_protection", 1, "minecraft:lava", false), 1.0);
    // Iron plating and maille bring their own projectile protection, so an arrow is measured against a helmet with an unrelated modifier.
    let iron_alone = protection_of("tconstruct:sharpness", 1, "minecraft:arrow", true);
    assert!(iron_alone > 0.0, "iron armor protects against projectiles by itself");
    assert_eq!(protection_of("tconstruct:fire_protection", 1, "minecraft:arrow", true), iron_alone, "fire protection adds nothing against an arrow");
    assert_eq!(protection_of("tconstruct:melee_protection", 1, "minecraft:mob_attack", false), 2.0);
    assert_eq!(protection_of("tconstruct:melee_protection", 1, "minecraft:mob_attack", true), 0.0, "an indirect hit is not melee protection");
    // Boundless raises the cap on summed protection: a level 1 adds 0.1 to the 0.8 attribute, 22.5 points.
    let cap_with = |level: i32| {
        let mut tool = ItemSlot::new(id("tconstruct:plate_helmet"), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(id("tconstruct:boundless"), level).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        crate::protection::protection_cap(crate::tool_stack::protection_cap(&tool.custom_data, &env))
    };
    assert!((cap_with(1) - 22.5).abs() < 1e-4, "{}", cap_with(1));
    assert!((cap_with(5) - 23.75).abs() < 1e-4, "the cap never passes 95% of 25");
    assert!((crate::protection::protection_cap(0.0) - 20.0).abs() < 1e-4);
    // The event comes after vanilla's protection: 10 damage with no vanilla protection and 5 points is 8 after the modifier.
    assert!((damage_after_protection(10.0, 0.0, 5.0, 20.0) - 8.0).abs() < 1e-5);
    // Vanilla's 5 points were already taken (8), so 10 points total leaves 6 rather than 4.8.
    assert!((damage_after_protection(8.0, 5.0, 10.0, 20.0) - 6.0).abs() < 1e-5);
}

#[test]
fn shipped_overwield_scales_an_attack_speed_attribute_with_the_tools_overslime() {
    use crate::{modifier_entries::List, tool_stack::{attribute_entries, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt, NbtCompound};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let amount_with = |overslime: Option<i32>| {
        let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse("tconstruct:overwield").unwrap(), 2).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        if let Some(amount) = overslime {
            let mut persistent = NbtCompound::new();
            persistent.insert("tconstruct:overslime".into(), Nbt::Int(amount));
            tool.custom_data.insert("tic_persistent".into(), Nbt::Compound(persistent));
        }
        let held = attribute_entries(&tool.custom_data, &env, &["mainhand"]);
        let speed = held.iter().find(|entry| entry["type"] == Nbt::String("minecraft:attack_speed".into()));
        speed.map(|entry| match entry["amount"] { Nbt::Double(amount) => amount, _ => panic!("amount is a double") })
    };
    assert_eq!(amount_with(None), Some(0.0), "no overslime adds nothing");
    assert!((amount_with(Some(1)).unwrap() - 0.15).abs() < 1e-5, "one point of overslime counts once");
    assert!((amount_with(Some(9)).unwrap() - 0.3).abs() < 1e-5, "the level (2) caps the overslime that counts");
}

#[test]
fn shipped_constant_enchantments_give_the_tool_enchantment_levels() {
    use crate::{modifier_entries::List, tool_stack::{enchantments, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let with = |modifier: &str, level: i32| {
        let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse(modifier).unwrap(), level).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        enchantments(&tool.custom_data, &env)
    };
    // Conditions on the tool's tags hold when there is no item context (a preview outside a stack).
    assert_eq!(with("tconstruct:respiration", 1).get("minecraft:respiration"), Some(&1));
    assert_eq!(with("tconstruct:swift_sneak", 1).get("minecraft:swift_sneak"), Some(&1));
    assert!(with("tconstruct:sharpness", 2).is_empty(), "a modifier without enchantment modules gives none");
}

#[test]
fn looting_enchants_a_melee_tool_by_its_level_and_severing_reports_its_level() {
    use crate::{modifier_entries::List, tool_stack::{enchantments, looting, rebuild_stats, severing_levels, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let sword = &world.tools.iter().find(|(id, _)| id.path() == "sword").unwrap().1;
    let sword_id = Identifier::parse("tconstruct:sword").unwrap();
    // The looting module's tool condition names the melee tag, so the held item and its tags are needed.
    let tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:sword"], &[("tconstruct:modifiable/melee", &["tconstruct:sword"])]).unwrap();
    let env = Environment { definition: Some(sword), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&sword_id), item_tags: Some(&tags) };
    let with = |modifier: &str, level: i32| {
        let mut tool = ItemSlot::new(sword_id.clone(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse(modifier).unwrap(), level).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        tool
    };
    // LevelingInt.EACH_LEVEL: the plain number in the module's data is the looting for each level of the modifier.
    assert_eq!(looting(&with("tconstruct:looting", 3).custom_data, &env), 3);
    assert_eq!(enchantments(&with("tconstruct:respiration", 2).custom_data, &env).get("minecraft:respiration"), Some(&2), "a constant enchantment's number is also for each level");
    assert_eq!(looting(&with("tconstruct:sharpness", 2).custom_data, &env), 0);
    assert_eq!(severing_levels(&with("tconstruct:severing", 2).custom_data, &env), vec![2.0]);
    assert!(severing_levels(&with("tconstruct:sharpness", 2).custom_data, &env).is_empty());
}

#[test]
fn shipped_severing_recipes_load_without_problems_and_the_creeper_gives_its_head() {
    let files = recursive_files("tools/severing", "tools/severing");
    assert_eq!(files.len(), 31, "the 38 original recipes less the 7 that read the entity's own state");
    let recipes = crate::severing::Recipes::load(&files);
    assert_eq!((recipes.len(), recipes.problems.len()), (31, 0), "{:?}", recipes.problems);
    let none = |_: &Identifier| false;
    let creeper = Identifier::parse("minecraft:creeper").unwrap();
    let mut drops = Vec::new();
    recipes.process(&creeper, &none, false, 1.0, 0.0, &mut drops, &mut || 0.0);
    assert!(drops.iter().any(|slot| slot.item.to_string() == "minecraft:creeper_head"), "{drops:?}");
}

#[test]
fn shipped_tool_recycling_loads_and_a_wood_pickaxe_breaks_into_its_parts() {
    use crate::{part_recycling::Recycles, tool_stack::{rebuild_stats, Environment, TAG_DAMAGE, TAG_MATERIALS}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let files = recursive_files("tools/recycling", "tools/recycling");
    assert_eq!(files.len(), 20, "the fifteen tool recycling recipes and the five damageable ones the station pack ships beside them");
    let recycles = Recycles::load(&files, &conditions());
    // The minotaur axe's record waits on another mod's condition, which never holds here.
    assert_eq!((recycles.len(), recycles.problems.len()), (14, 0), "{:?}", recycles.problems);
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
    tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
    assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
    let venom = Identifier::parse("tconstruct:venom_bottle").unwrap();
    // The general recipe takes any recyclable multipart tool; every tag query answers yes except the unrecyclable list.
    let in_tag = |_: &Identifier, tag: &Identifier| tag.path() != "modifiable/unrecyclable";
    let recipe = recycles.find(&tool, &venom, &in_tag).expect("a recycling recipe matches a plain pickaxe and the venom bottle");
    let registry = crate::tool_parts::registry();
    let parts = pickaxe.tool_parts();
    let buttons = recipe.buttons(&tool, parts, &registry, &world.materials);
    let names: Vec<String> = buttons.iter().map(|part| part.path().to_owned()).collect();
    assert_eq!(names, vec!["pick_head", "tool_handle", "tool_binding"], "the pickaxe's three parts, once each");
    let head = recipe.assemble(&tool, &buttons[0], parts, &registry, &world.materials).expect("the head");
    assert_eq!(head.item.path(), "pick_head");
    assert!(!head.custom_data.is_empty(), "the part carries the tool's material");
    // An undamaged tool always gives a second part, picked from the others.
    let extra = recipe.leftover(&tool, &buttons[0], parts, &registry, &world.materials, &mut || 0.0).expect("an undamaged tool gives a leftover part");
    assert!(extra.item.path() != "pick_head");
    // A worn tool loses that part with the share of durability it lost: a roll below the share gives nothing.
    tool.custom_data.insert(TAG_DAMAGE.into(), Nbt::Int(30));
    assert!(recipe.leftover(&tool, &buttons[0], parts, &registry, &world.materials, &mut || 0.1).is_none());
    assert!(recipe.leftover(&tool, &buttons[0], parts, &registry, &world.materials, &mut || 0.9).is_some());
    // A tool with upgrades does not recycle.
    let mut upgraded = tool.clone();
    upgraded.custom_data.insert(crate::tool_stack::TAG_UPGRADES.into(), crate::modifier_entries::List::default().with_modifier(Identifier::parse("tconstruct:sharpness").unwrap(), 1).unwrap().to_nbt());
    assert!(recycles.find(&upgraded, &venom, &in_tag).is_none());
}

#[test]
fn shipped_damageable_recycling_loads_and_a_worn_staff_recycles_by_its_remaining_durability() {
    use crate::damageable_recycling::Recycles;
    use pluto_lib::ItemSlot;
    let tools = recursive_files("tools/recycling", "tools/recycling");
    let tables = recursive_files("tables/recycling", "tables/recycling");
    assert_eq!((tools.len(), tables.len()), (20, 32));
    let recycles = Recycles::load_folders(&tools, &tables, &conditions());
    assert_eq!(recycles.len(), 20, "the five staff recipes and the fifteen unconditional tables recipes; the seventeen twilightforest tables recipes stay unloaded in a vanilla-only world, as their conditions say");
    assert!(recycles.problems().is_empty(), "{:#?}", recycles.problems());
    let in_tag = |_: &Identifier, _: &Identifier| false;
    let first_of = |source: &crate::fluid_ingredient::Source| match source {
        crate::fluid_ingredient::Source::Id(item) => Some(item.clone()),
        crate::fluid_ingredient::Source::Tag(_) => None,
    };
    let pattern = Identifier::parse("tconstruct:venom_bottle").unwrap();
    let crystal = Identifier::parse("tconstruct:crystal").unwrap();
    // The earth staff, undamaged: its results total five, so the crystal comes out whole and the
    // leftover budget buys the block result (the first other result in pattern order).
    let mut staff = ItemSlot::new(Identifier::parse("tconstruct:earth_staff").unwrap(), 1);
    let recipe = recycles.find(&staff, &pattern, 100, &in_tag).expect("the earth staff recipe matches an undamaged staff");
    assert_eq!(recipe.id.to_string(), "tconstruct:tools/recycling/earth_staff");
    let made = recipe.assemble(&crystal, &staff, 100, &first_of).expect("a crystal");
    assert_eq!((made.item.to_string(), made.count), ("tconstruct:earth_slime_crystal".to_string(), 2));
    let leftover = recipe.leftover(&crystal, &staff, 100, &first_of, &mut || 0.0).expect("a leftover");
    assert_eq!((leftover.item.to_string(), leftover.count), ("tconstruct:greenheart_log".to_string(), 2));
    // At a fifth of its durability left, the budget of five shrinks to one; nearly broken, the
    // recipe no longer matches at all.
    staff.damage = 80;
    let made = recipe.assemble(&crystal, &staff, 100, &first_of).expect("a crystal");
    assert_eq!(made.count, 1);
    staff.damage = 99;
    assert!(recycles.find(&staff, &pattern, 100, &in_tag).is_none(), "a budget of zero matches nothing");
    // The tables folder: a vanilla bow breaks into string and sticks.
    let bow = ItemSlot::new(Identifier::parse("minecraft:bow").unwrap(), 1);
    let recipe = recycles.find(&bow, &pattern, 384, &in_tag).expect("the bow recipe matches");
    let rod = recipe.assemble(&Identifier::parse("tconstruct:rod").unwrap(), &bow, 384, &first_of).expect("sticks");
    assert_eq!((rod.item.to_string(), rod.count), ("minecraft:stick".to_string(), 3));
}

#[test]
fn shipped_station_layouts_load_and_the_tab_order_matches_the_shipped_files() {
    use crate::station_layouts::Layouts;
    let files = files("station_layouts");
    assert_eq!(files.len(), 23);
    let layouts = Layouts::load(&files, &conditions());
    assert!(layouts.problems.is_empty(), "{:#?}", layouts.problems);
    for main in ["tinker_station", "tinkers_anvil", "scorched_anvil"] {
        assert_eq!(layouts.main_for(main).name.path(), main, "{main} is a main layout");
    }
    let tabs = layouts.tabs();
    assert_eq!(tabs.len(), 20);
    let names: Vec<String> = tabs.iter().map(|layout| layout.name.path().to_owned()).collect();
    let order: Vec<String> = crate::station_layout_order::TAB_ORDER.iter().map(|name| (*name).to_owned()).collect();
    assert_eq!(names, order, "the baked tab order is the shipped layouts' sort order");
}

#[test]
fn a_modifier_crystal_alone_applies_its_modifier_through_the_single_step_recipe() {
    use crate::{modifier_entries::List, modifier_recipes::{lone_crystal, Recipes}, tool_stack::{rebuild_stats, Environment, TAG_MATERIALS, TAG_PERSISTENT, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt, NbtCompound};
    let world = world();
    let files = recursive_files("tools/modifiers", "tools/modifiers");
    let recipes = Recipes::load(&files, &conditions());
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
    tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
    let mut slots = NbtCompound::new();
    slots.insert("upgrades".into(), Nbt::Int(3));
    tool.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(slots));
    assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
    let in_tag = |_: &Identifier, _: &Identifier| true;
    let sharp = Identifier::parse("tconstruct:sharpness").unwrap();
    let recipe = recipes.find_crystal(&tool.item, &sharp, &in_tag).expect("sharpness takes a crystal");
    let crafted = recipe.apply_crystal(&tool, &env, &in_tag).unwrap().expect("the crystal applies");
    assert_eq!(List::from_nbt(crafted.custom_data.get(TAG_UPGRADES)).level(&sharp), 1);
    // The table holds exactly one stack and it is a crystal: that is a crystal match; two stacks, or another item, is not.
    let crystal = Identifier::parse("tconstruct:modifier_crystal").unwrap();
    assert_eq!(lone_crystal(&[Some((crystal.clone(), 2, Some(sharp.clone()))), None]), Some((0, sharp.clone())));
    assert_eq!(lone_crystal(&[None, Some((crystal.clone(), 1, Some(sharp.clone())))]), Some((1, sharp.clone())));
    assert!(lone_crystal(&[Some((crystal.clone(), 1, Some(sharp.clone()))), Some((crystal.clone(), 1, Some(sharp.clone())))]).is_none());
    assert!(lone_crystal(&[Some((Identifier::parse("minecraft:stick").unwrap(), 1, None))]).is_none());
    assert!(lone_crystal(&[Some((crystal, 1, None))]).is_none(), "a crystal with no modifier is blank");
}

#[test]
fn shipped_maintained_gives_a_helmet_toughness_until_it_is_worn_below_half() {
    use crate::{modifier_entries::List, tool_stack::{attribute_entries, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let helmet = &world.tools.iter().find(|(id, _)| id.path() == "plate_helmet").unwrap().1;
    let helmet_id = Identifier::parse("tconstruct:plate_helmet").unwrap();
    let armor_tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:plate_helmet"], &[("tconstruct:modifiable/armor", &["tconstruct:plate_helmet"])]).unwrap();
    let env = Environment { definition: Some(helmet), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&helmet_id), item_tags: Some(&armor_tags) };
    let toughness_at = |damage: i32| {
        let mut tool = ItemSlot::new(helmet_id.clone(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse("tconstruct:maintained").unwrap(), 2).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        tool.custom_data.insert("Damage".into(), Nbt::Int(damage));
        let held = attribute_entries(&tool.custom_data, &env, &["head"]);
        println!("maintained entries at damage {damage}: {held:?}");
        held.iter().find(|entry| entry["type"] == Nbt::String("minecraft:armor_toughness".into()))
            .map(|entry| match entry["amount"] { Nbt::Double(amount) => amount, _ => panic!("amount is a double") })
    };
    // The bonus is the share of durability above half, so a fresh helmet gets the most and one worn below half gets none.
    let fresh = toughness_at(0);
    assert!(fresh.is_some_and(|amount| amount > 0.0), "a fresh helmet gains toughness: {fresh:?}");
    assert!(toughness_at(400).is_none_or(|amount| amount == 0.0), "a helmet worn below half its durability gains none");
}

#[test]
fn shipped_protection_that_names_the_attacker_asks_what_it_is_and_was_doing() {
    use crate::{damage_tags::DamageTags, melee_damage::{state_in_tag, STATE_NAMESPACE}, modifier_entries::List, protection::DamageSource, tool_stack::{protection, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{EntityState, ItemSlot, Nbt};
    let world = world();
    let helmet = &world.tools.iter().find(|(id, _)| id.path() == "plate_helmet").unwrap().1;
    let helmet_id = Identifier::parse("tconstruct:plate_helmet").unwrap();
    let armor_tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:plate_helmet"], &[("tconstruct:modifiable/armor", &["tconstruct:plate_helmet"])]).unwrap();
    let env = Environment { definition: Some(helmet), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&helmet_id), item_tags: Some(&armor_tags) };
    let (tags, problems) = DamageTags::load(&damage_tag_stacks());
    assert!(problems.is_empty(), "{problems:?}");
    let id = |text: &str| Identifier::parse(text).unwrap();
    let protection_against = |modifier: &str, attacker: &EntityState, undead: bool| {
        let mut tool = ItemSlot::new(helmet_id.clone(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(id(modifier), 2).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        let damage_type = id("minecraft:mob_attack");
        let in_tag = |tag: &Identifier| if tag.namespace() == STATE_NAMESPACE { state_in_tag(tag, Some(attacker), None) } else { undead && tag.to_string() == "minecraft:undead" };
        protection(&tool.custom_data, &env, &DamageSource { damage_type: &damage_type, indirect: false, tags: &tags, attacker_in_tag: &in_tag, entities: Default::default() }, 0.0)
    };
    let calm = EntityState { kind: "minecraft:husk".into(), on_ground: true, ..EntityState::default() };
    let burning = EntityState { fire_ticks: 40, ..calm.clone() };
    let flying = EntityState { on_ground: false, ..calm.clone() };
    assert_eq!(protection_against("tconstruct:tempered_protection", &burning, false), 2.5, "against a burning attacker");
    assert_eq!(protection_against("tconstruct:tempered_protection", &calm, false), 0.0, "against a calm one");
    assert_eq!(protection_against("tconstruct:consecrated", &calm, true), 2.5, "against the undead");
    assert_eq!(protection_against("tconstruct:consecrated", &calm, false), 0.0, "against the living");
    assert_eq!(protection_against("tconstruct:airborn", &flying, false), 2.5, "against an attacker in the air");
    assert_eq!(protection_against("tconstruct:airborn", &calm, false), 0.0, "against one on the ground");
}

#[test]
fn shipped_decay_puts_wither_on_what_a_pickaxe_hits_and_on_what_hits_its_wearer() {
    use crate::{modifier_entries::List, tool_stack::{counter_effects, rebuild_stats, weapon_effects, Environment, TAG_MATERIALS, TAG_UPGRADES}, weapon_effect::Whom};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let decay = |level: i32| {
        let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse("tconstruct:decay").unwrap(), level).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        tool
    };
    let tool = decay(2);
    let none = |_: &Identifier| false;
    // The weapon module: wither at the modifier level (per_level 1), 120 ticks, always.
    // (Decay also brings pierce along, an effect of the original's own that no registration here carries.)
    let hit = weapon_effects(&tool.custom_data, &env, &none, &mut || 0.0);
    let wither: Vec<_> = hit.iter().filter(|(_, grant)| grant.effect.to_string() == "minecraft:wither").collect();
    assert_eq!(wither.len(), 1, "{hit:?}");
    assert_eq!((wither[0].1.amplifier, wither[0].1.duration), (1, 120));
    // The armor counter: wither II for 120 ticks on the attacker with 50% a level (certain at level 2), and a wither I on the wearer at 10% a level.
    let counter = counter_effects(&tool.custom_data, &env, &none, &mut || 0.0);
    let counter: Vec<_> = counter.into_iter().filter(|(_, grant)| grant.effect.to_string() == "minecraft:wither").collect();
    let on_attacker: Vec<_> = counter.iter().filter(|(whom, _)| *whom == Whom::Attacker).collect();
    let on_wearer: Vec<_> = counter.iter().filter(|(whom, _)| *whom == Whom::Wearer).collect();
    assert_eq!(on_attacker.len(), 1, "{counter:?}");
    assert_eq!((on_attacker[0].1.amplifier, on_attacker[0].1.duration), (1, 120));
    assert_eq!(on_wearer.len(), 1, "a low roll lands the wearer's wither too: {counter:?}");
    assert_eq!(on_wearer[0].1.amplifier, 0);
    let unlucky = counter_effects(&tool.custom_data, &env, &none, &mut || 0.99);
    let unlucky: Vec<_> = unlucky.into_iter().filter(|(_, grant)| grant.effect.to_string() == "minecraft:wither").collect();
    assert_eq!(unlucky.iter().filter(|(whom, _)| *whom == Whom::Wearer).count(), 0, "a high roll spares the wearer (20% at level 2)");
    assert_eq!(unlucky.iter().filter(|(whom, _)| *whom == Whom::Attacker).count(), 1, "the attacker's 100% at level 2 needs no roll");
}

#[test]
fn shipped_decay_and_magnetic_put_their_effect_on_the_holder_of_a_swing_a_shot_and_for_magnetic_a_shearing() {
    use crate::{modifier_entries::List, tool_stack::{rebuild_stats, usage_effects, Environment, TAG_MATERIALS, TAG_UPGRADES}, weapon_effect::Usage};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    for id in ["tconstruct:decay", "tconstruct:magnetic"] {
        assert!(world.registry.get(&Identifier::parse(id).unwrap()).is_some_and(|modifier| modifier.unbound.is_empty()), "{id} is fully bound");
    }
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let tool_with = |modifier: &str, level: i32| {
        let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse(modifier).unwrap(), level).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        tool
    };
    let effects = |tool: &ItemSlot, usage: Usage, roll: f32| usage_effects(&tool.custom_data, &env, usage, &mut || roll)
        .into_iter().map(|grant| (grant.effect.to_string(), grant.amplifier, grant.duration)).collect::<Vec<_>>();
    let wither = |amplifier| vec![("minecraft:wither".to_string(), amplifier, 120)];
    // decay: wither at the level, 120 ticks, a quarter of the time; swung or shot, not for shearing (it names no shear hook).
    let decay = tool_with("tconstruct:decay", 2);
    assert_eq!(effects(&decay, Usage::MeleeHit, 0.1), wither(1));
    assert_eq!(effects(&decay, Usage::Launch { primary: true }, 0.1), wither(1));
    assert!(effects(&decay, Usage::MeleeHit, 0.3).is_empty(), "a roll over 25% spares the holder");
    assert!(effects(&decay, Usage::Shear { target: true }, 0.0).is_empty());
    assert!(effects(&decay, Usage::Launch { primary: false }, 0.0).is_empty(), "only the primary arrow of a shot");
    // magnetic: always, for every use (its hooks default to all): the original's own effect at level 1 for 40 ticks.
    let magnetic = tool_with("tconstruct:magnetic", 1);
    for usage in [Usage::MeleeHit, Usage::Launch { primary: true }, Usage::Shear { target: true }] {
        assert_eq!(effects(&magnetic, usage, 0.99), vec![("tconstruct:magnetic".to_string(), 0, 40)], "{usage:?}");
    }
    // A tool with neither has nothing to put on its holder.
    assert!(usage_effects(&tool_with("tconstruct:sharpness", 1).custom_data, &env, Usage::MeleeHit, &mut || 0.0).is_empty());
}

#[test]
fn shipped_fiery_freezing_and_springy_act_on_the_mob_hit_and_on_the_mob_that_hits_the_wearer() {
    use crate::{modifier_entries::List, tool_stack::{attack_actions, counter_actions, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ActionTarget, EntityAction, ItemSlot, KnockbackDirection, Nbt};
    let world = world();
    let tool_with = |tool: &str, material: &str, parts: usize, upgrades: &[(&str, i32)]| {
        let definition = &world.tools.iter().find(|(id, _)| id.path() == tool).unwrap().1;
        let item = Identifier::parse(&format!("tconstruct:{tool}")).unwrap();
        let tags = pluto_lib::fixtures::registry_with_tags(&[&item.to_string()], &[("tconstruct:modifiable/armor", &[item.to_string().as_str()][..])]).unwrap();
        let env = Environment { definition: Some(definition), has_materials: true, materials: &world.materials, modifiers: &world.registry,
            item_tags_loaded: true, item: Some(&item), item_tags: Some(&tags) };
        let mut stack = ItemSlot::new(item.clone(), 1);
        stack.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(vec![Nbt::String(material.into()); parts]));
        let mut list = List::default();
        for (id, level) in upgrades { list = list.with_modifier(Identifier::parse(id).unwrap(), *level).unwrap(); }
        stack.custom_data.insert(TAG_UPGRADES.into(), list.to_nbt());
        assert!(rebuild_stats(&mut stack.custom_data, &env).unwrap());
        (stack, tags)
    };
    let (helmet, helmet_tags) = tool_with("plate_helmet", "tconstruct:iron", 2, &[("tconstruct:fiery", 2), ("tconstruct:freezing", 1), ("tconstruct:springy", 2)]);
    let helmet_definition = &world.tools.iter().find(|(id, _)| id.path() == "plate_helmet").unwrap().1;
    let helmet_id = Identifier::parse("tconstruct:plate_helmet").unwrap();
    let env = Environment { definition: Some(helmet_definition), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&helmet_id), item_tags: Some(&helmet_tags) };
    let none = |_: &Identifier| false;
    let attacker = ActionTarget::Attacker;
    // Armor that counters: a low roll lands all three (30%, 15% and 30% at these levels); fire burns the constant second, freezing adds
    // two seconds of 40 ticks, and springy pushes the constant half against the way the mob faces.
    let landed = counter_actions(&helmet.custom_data, &env, &none, &mut || 0.0);
    assert!(landed.contains(&EntityAction::SetFire { target: attacker, seconds: 1 }), "{landed:?}");
    assert!(landed.contains(&EntityAction::Freeze { target: attacker, ticks: 80 }), "{landed:?}");
    assert!(landed.contains(&EntityAction::Knockback { target: attacker, strength: 0.5, direction: KnockbackDirection::AgainstFacing }), "{landed:?}");
    assert_eq!(landed.len(), 3, "{landed:?}");
    assert!(counter_actions(&helmet.custom_data, &env, &none, &mut || 0.99).is_empty(), "a high roll lands none of them");
    // A tool that is not armor counters nothing: the counters ask for the armor tag.
    let (pickaxe, _) = tool_with("pickaxe", "tconstruct:wood", 3, &[("tconstruct:fiery", 2), ("tconstruct:freezing", 1)]);
    let pickaxe_definition = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let pickaxe_env = Environment { definition: Some(pickaxe_definition), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    assert!(counter_actions(&pickaxe.custom_data, &pickaxe_env, &none, &mut || 0.0).is_empty());
    // The hit: five seconds of fire a level, and (4 + 4 a level) seconds of freezing at 40 ticks each.
    let hit = attack_actions(&pickaxe.custom_data, &pickaxe_env);
    assert!(hit.contains(&EntityAction::SetFire { target: ActionTarget::Victim, seconds: 10 }), "{hit:?}");
    assert!(hit.contains(&EntityAction::Freeze { target: ActionTarget::Victim, ticks: 320 }), "{hit:?}");
}

#[test]
fn shipped_block_damage_modules_block_the_sources_they_name() {
    use crate::{damage_tags::DamageTags, modifier_entries::List, protection::DamageSource, tool_stack::{blocks_damage, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let boots = &world.tools.iter().find(|(id, _)| id.path() == "plate_boots").unwrap().1;
    let boots_id = Identifier::parse("tconstruct:plate_boots").unwrap();
    let armor_tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:plate_boots"], &[("tconstruct:modifiable/armor", &["tconstruct:plate_boots"])]).unwrap();
    let env = Environment { definition: Some(boots), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&boots_id), item_tags: Some(&armor_tags) };
    let (tags, problems) = DamageTags::load(&damage_tag_stacks());
    assert!(problems.is_empty(), "{problems:?}");
    let blocked = |modifier: &str, damage_type: &str| {
        let mut tool = ItemSlot::new(boots_id.clone(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse(modifier).unwrap(), 1).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        let damage_type = Identifier::parse(damage_type).unwrap();
        blocks_damage(&tool.custom_data, &env, &DamageSource { damage_type: &damage_type, indirect: false, tags: &tags, attacker_in_tag: &|_| false, entities: Default::default() })
    };
    assert!(blocked("tconstruct:long_fall", "minecraft:fall"), "long fall blocks fall damage");
    assert!(!blocked("tconstruct:long_fall", "minecraft:mob_attack"), "and nothing else");
    assert!(blocked("tconstruct:frost_walker", "minecraft:hot_floor"), "frost walker blocks magma damage");
    assert!(!blocked("tconstruct:frost_walker", "minecraft:fall"), "and not a fall");
}

#[test]
fn shipped_projectile_protection_and_magnet_take_the_largest_and_the_total_over_worn_boots() {
    use crate::{max_armor, modifier_entries::List, tool_stack::{armor_levels, max_armor_claims, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let boots = &world.tools.iter().find(|(id, _)| id.path() == "plate_boots").unwrap().1;
    let boots_id = Identifier::parse("tconstruct:plate_boots").unwrap();
    let env = Environment { definition: Some(boots), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&boots_id), item_tags: None };
    let piece = |modifier: &str, level: i32| {
        let mut tool = ItemSlot::new(boots_id.clone(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse(modifier).unwrap(), level).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        tool
    };
    let (low, high) = (piece("tconstruct:projectile_protection", 1), piece("tconstruct:projectile_protection", 3));
    let claims: Vec<_> = [&low, &high].iter().flat_map(|tool| max_armor_claims(&tool.custom_data, &env)).collect();
    assert_eq!(claims.len(), 2, "{claims:?}");
    let amounts = max_armor::strongest(claims);
    assert_eq!(amounts.len(), 1, "{amounts:?}");
    assert_eq!(amounts[0].0, "minecraft:knockback_resistance");
    // The plate boots carry two levels of their own, so the pieces are at levels 3 and 5: the larger counts once, not 8.
    assert!((amounts[0].1 - 0.25).abs() < 1e-6, "{amounts:?}");
    // The attributes the host lacks leave the other max armor modifiers unbound, so they claim nothing.
    let dragon = max_armor_claims(&piece("tconstruct:dragonborn", 2).custom_data, &env);
    assert!(dragon.iter().all(|claim| format!("{claim:?}").contains("projectile_protection")), "only the boots' own projectile protection claims: {dragon:?}");
    // Armor levels add up over the pieces under their key.
    let (a, b) = (piece("tconstruct:magnetic", 1), piece("tconstruct:magnetic", 2));
    let totals = max_armor::totals([&a, &b].iter().flat_map(|tool| armor_levels(&tool.custom_data, &env, "feet")));
    assert_eq!(totals[&Identifier::parse("tconstruct:magnet").unwrap()], 3);
    let belt = piece("tconstruct:soul_belt", 1);
    assert_eq!(armor_levels(&belt.custom_data, &env, "feet"), vec![(Identifier::parse("tconstruct:soul_belt").unwrap(), 1)]);
}

#[test]
fn shipped_inventory_modifiers_give_a_tool_its_slots_filters_and_stored_stacks() {
    use crate::{modifier_entries::List, tool_inventory::{Items, MenuKey, TOTAL_SLOTS}, tool_stack::{inventory, inventory_slots, rebuild_stats, with_persistent, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let boots = &world.tools.iter().find(|(id, _)| id.path() == "plate_boots").unwrap().1;
    let boots_id = Identifier::parse("tconstruct:plate_boots").unwrap();
    let armor_tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:plate_boots"], &[("tconstruct:modifiable/armor", &["tconstruct:plate_boots"])]).unwrap();
    let env = Environment { definition: Some(boots), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&boots_id), item_tags: Some(&armor_tags) };
    let build = |mods: &[(&str, i32)]| {
        let mut tool = ItemSlot::new(boots_id.clone(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
        let mut list = List::default();
        for (modifier, level) in mods { list = list.with_modifier(Identifier::parse(modifier).unwrap(), *level).unwrap(); }
        tool.custom_data.insert(TAG_UPGRADES.into(), list.to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        tool
    };
    let id = |text: &str| Identifier::parse(text).unwrap();
    let max_stack = |_: &Identifier| 64;
    let in_tag = |item: &Identifier, tag: &Identifier| (tag.path() == "slimeball_ammo" && item.path() == "slime_ball") || (tag.path() == "throwable" && item.path() == "snowball") || (tag.path() == "arrows" && matches!(item.path(), "arrow" | "tipped_arrow"));
    let fits = |_: &Identifier| true;
    let no_handler = |_: &ItemSlot| false;
    let items = Items { max_stack: &max_stack, in_tag: &in_tag, can_fit_inside_container: &fits, has_item_handler: &no_handler };
    // The pocket gives 3 slots a level under its own key; the tool belt 3 plus one a level.
    let pockets = build(&[("tconstruct:pocket", 2)]);
    assert_eq!(inventory_slots(&pockets.custom_data), 6);
    assert_eq!(inventory(&pockets.custom_data, &env).slots(), 6);
    let belt = build(&[("tconstruct:tool_belt", 1)]);
    assert_eq!(inventory_slots(&belt.custom_data), 4);
    // Every modifier counts, and the last one's slots come first.
    let both = build(&[("tconstruct:pocket", 1), ("tconstruct:tool_belt", 1)]);
    assert_eq!(inventory_slots(&both.custom_data), 7);
    let Some(Nbt::Compound(volatile)) = both.custom_data.get("tic_volatile_data") else { panic!("no volatile data") };
    assert_eq!(volatile.get(TOTAL_SLOTS), Some(&Nbt::Int(7)));
    let handler = inventory(&both.custom_data, &env);
    let mut tool = both.clone();
    with_persistent(&mut tool.custom_data, |persistent| {
        let rest = handler.insert_item(&items, persistent, 0, ItemSlot::new(id("minecraft:stick"), 70), false);
        assert_eq!(rest.map(|rest| rest.count), Some(6), "a slot holds a stack of 64 by default");
        handler.insert_item(&items, persistent, 6, ItemSlot::new(id("minecraft:apple"), 1), false);
    });
    let Some(Nbt::Compound(persistent)) = tool.custom_data.get("tic_persistent") else { panic!("no persistent data") };
    assert!(persistent.contains_key("tconstruct:tool_belt") && persistent.contains_key("tconstruct:pockets"), "each modifier's stacks sit under its own key");
    // Filters and limits come from the data.
    let sliver = inventory(&build(&[("tconstruct:sliver", 1)]).custom_data, &env);
    assert!(sliver.is_item_valid(&items, 0, &ItemSlot::new(id("minecraft:slime_ball"), 1)) && !sliver.is_item_valid(&items, 0, &ItemSlot::new(id("minecraft:stick"), 1)));
    assert_eq!(sliver.slot_limit(0), 32);
    assert_eq!(sliver.pattern(0, false).map(ToString::to_string).as_deref(), Some("tconstruct:slimeball"));
    assert_eq!(sliver.pattern(0, true), None);
    let sleeves = inventory(&build(&[("tconstruct:sleeves", 1)]).custom_data, &env);
    assert!(sleeves.is_item_valid(&items, 2, &ItemSlot::new(id("minecraft:snowball"), 1)) && sleeves.slot_limit(2) == 16);
    let minimap = inventory(&build(&[("tconstruct:minimap", 1)]).custom_data, &env);
    assert!(minimap.is_item_valid(&items, 0, &ItemSlot::new(id("minecraft:filled_map"), 1)) && !minimap.is_item_valid(&items, 0, &ItemSlot::new(id("minecraft:paper"), 1)));
    assert_eq!(minimap.slot_limit(0), 1);
    let frame = inventory(&build(&[("tconstruct:item_frame", 2)]).custom_data, &env);
    assert_eq!((frame.slots(), frame.slot_limit(5)), (6, 1));
    assert_eq!(inventory(&build(&[("tconstruct:shield_strap", 1)]).custom_data, &env).slots(), 1);
    // The menu modules are bound, and so are the quiver inventories: the bulk quiver gives two slots a level
    // that take arrows (the boots are no crossbow or ballista, so nothing else is ammunition for them).
    let keys = |modifier: &str| world.registry.get(&id(modifier)).map(|modifier| modifier.inventory_menu_keys()).unwrap_or_default();
    assert_eq!(keys("tconstruct:pocket"), vec![MenuKey::Any]);
    assert_eq!(keys("tconstruct:shield_strap"), vec![MenuKey::Shift]);
    assert_eq!(keys("tconstruct:tool_belt"), vec![MenuKey::Shift]);
    assert_eq!(keys("tconstruct:crafting_table"), vec![MenuKey::Any]);
    assert_eq!(inventory_slots(&build(&[("tconstruct:bulk_quiver", 1)]).custom_data), 2);
    let quiver = inventory(&build(&[("tconstruct:bulk_quiver", 1)]).custom_data, &env);
    assert!(quiver.is_item_valid(&items, 0, &ItemSlot::new(id("minecraft:arrow"), 1)) && !quiver.is_item_valid(&items, 0, &ItemSlot::new(id("minecraft:stick"), 1)));
    assert!(!quiver.is_item_valid(&items, 0, &ItemSlot::new(id("minecraft:firework_rocket"), 1)));
    assert_eq!(inventory_slots(&build(&[("tconstruct:trick_quiver", 1)]).custom_data), 3);
}

/// A rebuilt wood pickaxe carrying `modifier` at `level` (no item context, so every stat is supported).
fn stat_tool(world: &World, modifier: &str, level: i32) -> pluto_lib::NbtCompound {
    use crate::{modifier_entries::List, tool_stack::{rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::Nbt;
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let mut nbt = pluto_lib::NbtCompound::new();
    nbt.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
    nbt.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse(modifier).unwrap(), level).unwrap().to_nbt());
    assert!(rebuild_stats(&mut nbt, &env).unwrap());
    nbt
}
#[test]
fn shipped_conditional_stat_and_power_bind_exactly_the_modules_whose_inputs_are_reported() {
    let mut registry = Registry::default();
    registry.reload(&files("modifiers"), &conditions());
    let unbound = |name: &str, kind: &str| registry.get(&Identifier::parse(&format!("tconstruct:{name}")).unwrap()).unwrap().unbound.iter().any(|id| id.to_string() == kind);
    // Holder state (airborne), tool data (durability, stat, persistent data) and the projectile data are held.
    for name in ["airborne", "dragonshot", "maintained", "overwield", "war_charge"] { assert!(!unbound(name, "tconstruct:conditional_stat"), "{name}"); }
    // Health, an effect level and the biome temperature of the holder are reported with the hit.
    for name in ["ambush", "antitoxin", "flamestance", "invariant", "raging", "temperate"] { assert!(!unbound(name, "tconstruct:conditional_stat"), "{name}"); }
    // So are the health, rain and armor coverage of the target.
    for name in ["fins", "holy", "rebound", "ambush", "shock", "valiant"] { assert!(!unbound(name, "tconstruct:conditional_power"), "{name}"); }
    // The melee modules over the same facts, and the damage adjustment over the wearer health.
    for name in ["ambush", "antitoxin", "flamestance", "invariant", "raging", "shock", "valiant"] { assert!(!unbound(name, "tconstruct:conditional_melee_damage"), "{name}"); }
    for name in ["warded", "dragonheart"] { assert!(!unbound(name, "tconstruct:adjust_damage"), "{name}"); }
}
#[test]
fn shipped_conditional_stats_follow_the_holder_state_and_the_tool_data() {
    use crate::{melee_damage::state_in_tag, tool_stack::{conditional_stat, durability, Environment, TAG_DAMAGE, TAG_PERSISTENT}, tool_stats::Stat};
    use pluto_lib::{EntityState, Nbt, NbtCompound};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let value = |tool: &NbtCompound, stat: Stat, holder: &EntityState| conditional_stat(tool, &env, stat, &|tag| state_in_tag(tag, Some(holder), None), &crate::entity_facts::Entities { attacker: Some(holder), target: None });
    let grounded = EntityState { on_ground: true, ..EntityState::default() };
    let jumping = EntityState { on_ground: false, ..EntityState::default() };
    let swimming = EntityState { on_ground: false, in_water: true, ..EntityState::default() };
    // airborne: +0.1 velocity while in the air (not in water).
    let airborne = stat_tool(&world, "tconstruct:airborne", 1);
    assert_eq!(value(&airborne, Stat::Velocity, &grounded), 1.0);
    assert!((value(&airborne, Stat::Velocity, &jumping) - 1.1).abs() < 1e-6);
    assert_eq!(value(&airborne, Stat::Velocity, &swimming), 1.0, "AIRBORNE excludes swimming");
    assert_eq!(value(&airborne, Stat::DrawSpeed, &jumping), 1.0, "another stat is left alone");
    // dragonshot: +1 projectile damage per level while airborne.
    let dragonshot = stat_tool(&world, "tconstruct:dragonshot", 2);
    assert_eq!(value(&dragonshot, Stat::ProjectileDamage, &jumping), 4.0);
    assert_eq!(value(&dragonshot, Stat::ProjectileDamage, &grounded), 2.0);
    // overwield: 0.06 per point of overslime, up to the modifier level, on velocity and draw speed.
    let mut overwield = stat_tool(&world, "tconstruct:overwield", 2);
    let mut persistent = NbtCompound::new();
    persistent.insert("tconstruct:overslime".into(), Nbt::Int(5));
    overwield.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
    assert!((value(&overwield, Stat::Velocity, &grounded) - 1.12).abs() < 1e-6);
    assert!((value(&overwield, Stat::DrawSpeed, &grounded) - 1.12).abs() < 1e-6);
    // war charge: 0.01 velocity per point of stored charge.
    let mut charged = stat_tool(&world, "tconstruct:war_charge", 1);
    let mut persistent = NbtCompound::new();
    persistent.insert("tconstruct:war_charge".into(), Nbt::Float(50.0));
    charged.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
    assert!((value(&charged, Stat::Velocity, &grounded) - 1.5).abs() < 1e-6);
    // maintained: 0.08 velocity per level, scaled by the share of durability above half.
    let mut maintained = stat_tool(&world, "tconstruct:maintained", 2);
    assert!((value(&maintained, Stat::Velocity, &grounded) - 1.16).abs() < 1e-5, "{}", value(&maintained, Stat::Velocity, &grounded));
    let worn = durability(&maintained) * 3 / 4;
    maintained.insert(TAG_DAMAGE.into(), Nbt::Int(worn));
    assert_eq!(value(&maintained, Stat::Velocity, &grounded), 1.0, "below half the bonus is gone");
}
#[test]
fn shipped_conditional_power_follows_the_target_and_the_projectile() {
    use crate::{conditional_stat::PowerContext, melee_damage::state_in_tag, tool_stack::{power_hit, power_shoot, Environment}};
    use pluto_lib::{EntityState, NbtCompound};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let shot = |modifier: &str, level: i32, target: EntityState, tags: &'static [&'static str], launch: &dyn Fn(&mut NbtCompound)| {
        let tool = stat_tool(&world, modifier, level);
        let mut arrow = NbtCompound::new();
        power_shoot(&tool, &env, false, &mut arrow);
        launch(&mut arrow);
        let target_in_tag = |tag: &Identifier| tags.contains(&tag.to_string().as_str()) || state_in_tag(tag, None, Some(&target));
        let context = PowerContext { holder_in_tag: &|_| false, target_in_tag: &target_in_tag, entities: crate::entity_facts::Entities { attacker: None, target: Some(&target) } };
        power_hit(&tool, &env, &mut arrow, &context, 2.0)
    };
    let dry = EntityState::default();
    let wet = EntityState { in_water: true, ..EntityState::default() };
    let nothing = |_: &mut NbtCompound| {};
    assert_eq!(shot("tconstruct:fins", 2, wet, &[], &nothing), 4.0, "fins: +1 power per level against a target in water");
    assert_eq!(shot("tconstruct:fins", 2, dry.clone(), &[], &nothing), 2.0);
    assert_eq!(shot("tconstruct:holy", 2, dry.clone(), &["minecraft:undead"], &nothing), 3.5, "holy: +0.75 per level against the undead");
    assert_eq!(shot("tconstruct:holy", 2, dry.clone(), &[], &nothing), 2.0);
    // rebound: half a point per bounce and level, from the bounce count of the arrow.
    let bounced = |arrow: &mut NbtCompound| { arrow.insert("tconstruct:bounce".into(), pluto_lib::Nbt::Float(3.0)); };
    assert_eq!(shot("tconstruct:rebound", 2, dry, &[], &bounced), 5.0);
}

/// The reported facts of an entity in a biome of 0.8, at `health` of 20.
fn facts(health: f32) -> pluto_lib::EntityState {
    pluto_lib::EntityState { health: Some(health), max_health: Some(20.0), biome_temperature: Some(0.8), ..pluto_lib::EntityState::default() }
}

#[test]
fn shipped_melee_modules_read_health_temperature_effect_level_water_and_armor() {
    use crate::{entity_facts::Entities, melee_damage::state_in_tag, tool_stack::{melee_damage, Environment}};
    use pluto_lib::EntityState;
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    // A hit of damage 10 with `modifier` at `level`; the attacker and target as PlutoLib would report them.
    let hit = |modifier: &str, level: i32, attacker: Option<&EntityState>, target: Option<&EntityState>| {
        let tool = stat_tool(&world, modifier, level);
        melee_damage(&tool, &env, 2.0, 10.0, &|tag: &Identifier| tag.namespace() == crate::melee_damage::STATE_NAMESPACE && state_in_tag(tag, attacker, target), &Entities { attacker, target })
    };
    let near = |left: f32, right: f32| assert!((left - right).abs() < 1e-4, "{left} != {right}");
    // raging: (11 - health) / 10 clamped to 0..1, times 2.5 a level.
    near(hit("tconstruct:raging", 2, Some(&facts(1.0)), None), 15.0);
    near(hit("tconstruct:raging", 2, Some(&facts(6.0)), None), 12.5);
    near(hit("tconstruct:raging", 2, Some(&facts(20.0)), None), 10.0);
    near(hit("tconstruct:raging", 2, None, None), 15.0); // no attacker reported: the fallback health is 0
    // ambush: a level of damage for an attacker at full health plus 0.6 a level for a target at full health.
    let hurt = facts(5.0);
    near(hit("tconstruct:ambush", 2, Some(&facts(20.0)), Some(&facts(20.0))), 13.2);
    near(hit("tconstruct:ambush", 2, Some(&facts(20.0)), Some(&hurt)), 12.0);
    near(hit("tconstruct:ambush", 2, Some(&hurt), Some(&facts(20.0))), 11.2);
    near(hit("tconstruct:ambush", 2, Some(&hurt), Some(&hurt)), 10.0);
    near(hit("tconstruct:ambush", 2, None, None), 13.2); // neither reported: the fallbacks are 1 and 0.6, a full-health pair
    // flamestance: the attacker's biome temperature above 0.75 adds damage; the fallback of 2 applies when it is not known.
    let desert = EntityState { biome_temperature: Some(2.0), ..EntityState::default() };
    let tundra = EntityState { biome_temperature: Some(0.0), ..EntityState::default() };
    near(hit("tconstruct:flamestance", 1, Some(&desert), None), 11.25);
    near(hit("tconstruct:flamestance", 1, Some(&tundra), None), 9.25);
    near(hit("tconstruct:flamestance", 1, None, None), 11.25);
    // invariant: 1.28 a level at 0.75 and less the further the temperature is from it.
    let mild = EntityState { biome_temperature: Some(0.75), ..EntityState::default() };
    near(hit("tconstruct:invariant", 1, Some(&mild), None), 11.6);
    near(hit("tconstruct:invariant", 1, Some(&desert), None), 10.0);
    // antitoxin: only while poisoned, (effect level + 0.5) a level.
    let poisoned = EntityState { effects: vec!["minecraft:poison".into()], effect_levels: vec![("minecraft:poison".into(), 1)], ..EntityState::default() };
    near(hit("tconstruct:antitoxin", 1, Some(&poisoned), None), 12.5);
    near(hit("tconstruct:antitoxin", 1, Some(&EntityState::default()), None), 10.0);
    // shock: 2 a level against a target in water, 1 in the rain, none when dry.
    let wet = EntityState { in_water: true, ..EntityState::default() };
    let diver = EntityState { eyes_in_water: true, ..EntityState::default() };
    let rained_on = EntityState { rain_exposed: true, ..EntityState::default() };
    near(hit("tconstruct:shock", 2, None, Some(&wet)), 14.0);
    near(hit("tconstruct:shock", 2, None, Some(&diver)), 14.0);
    near(hit("tconstruct:shock", 2, None, Some(&rained_on)), 12.0);
    near(hit("tconstruct:shock", 2, None, Some(&EntityState::default())), 10.0);
    near(hit("tconstruct:shock", 2, None, None), 14.0);
    // valiant: 1.6 a level times the share of the target armor slots that are filled (the fallback is a full set).
    let armored = EntityState { armor_coverage: Some(0.5), ..EntityState::default() };
    near(hit("tconstruct:valiant", 1, None, Some(&armored)), 10.8);
    near(hit("tconstruct:valiant", 1, None, None), 11.6);
}

#[test]
fn shipped_stat_and_power_modules_read_the_holder_and_target_facts() {
    use crate::{conditional_stat::PowerContext, entity_facts::Entities, melee_damage::state_in_tag, tool_stack::{conditional_stat, power_hit, power_shoot, Environment}, tool_stats::Stat};
    use pluto_lib::{EntityState, NbtCompound};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let near = |left: f32, right: f32| assert!((left - right).abs() < 1e-4, "{left} != {right}");
    let stat = |modifier: &str, level: i32, stat: Stat, holder: &EntityState| {
        let tool = stat_tool(&world, modifier, level);
        conditional_stat(&tool, &env, stat, &|tag| state_in_tag(tag, Some(holder), None), &Entities { attacker: Some(holder), target: None })
    };
    // raging: projectile damage (base 2) rises by a level as health falls to 1.
    near(stat("tconstruct:raging", 2, Stat::ProjectileDamage, &facts(1.0)), 4.0);
    near(stat("tconstruct:raging", 2, Stat::ProjectileDamage, &facts(20.0)), 2.0);
    // ambush: half a point of projectile damage a level at full health.
    near(stat("tconstruct:ambush", 2, Stat::ProjectileDamage, &facts(20.0)), 3.0);
    near(stat("tconstruct:ambush", 2, Stat::ProjectileDamage, &facts(3.0)), 2.0);
    // temperate: draw speed rises as the biome drops below 0.75.
    near(stat("tconstruct:temperate", 1, Stat::DrawSpeed, &EntityState { biome_temperature: Some(0.2), ..EntityState::default() }), 1.0 + 0.55 * 0.12);
    near(stat("tconstruct:temperate", 1, Stat::DrawSpeed, &EntityState { biome_temperature: Some(1.5), ..EntityState::default() }), 1.0);
    // antitoxin: a poisoned holder draws faster.
    let poisoned = EntityState { effects: vec!["minecraft:poison".into()], effect_levels: vec![("minecraft:poison".into(), 0)], ..EntityState::default() };
    near(stat("tconstruct:antitoxin", 1, Stat::DrawSpeed, &poisoned), 1.0 + 1.5 * 0.1);
    near(stat("tconstruct:antitoxin", 1, Stat::DrawSpeed, &EntityState::default()), 1.0);
    // Power against a target.
    let shot = |modifier: &str, level: i32, target: &EntityState| {
        let tool = stat_tool(&world, modifier, level);
        let mut arrow = NbtCompound::new();
        power_shoot(&tool, &env, false, &mut arrow);
        let target_in_tag = |tag: &Identifier| state_in_tag(tag, None, Some(target));
        let context = PowerContext { holder_in_tag: &|_| false, target_in_tag: &target_in_tag, entities: Entities { attacker: None, target: Some(target) } };
        power_hit(&tool, &env, &mut arrow, &context, 2.0)
    };
    near(shot("tconstruct:shock", 2, &EntityState { in_water: true, ..EntityState::default() }), 4.0);
    near(shot("tconstruct:shock", 2, &EntityState { rain_exposed: true, ..EntityState::default() }), 3.0);
    near(shot("tconstruct:shock", 2, &EntityState::default()), 2.0);
    near(shot("tconstruct:valiant", 2, &EntityState { armor_coverage: Some(0.5), ..EntityState::default() }), 2.8);
    near(shot("tconstruct:ambush", 2, &facts(20.0)), 2.5);
    near(shot("tconstruct:ambush", 2, &facts(9.0)), 2.0);
}

#[test]
fn shipped_warded_and_dragonheart_adjust_the_damage_by_the_wearers_health() {
    use crate::{damage_tags::DamageTags, entity_facts::Entities, modifier_entries::List, protection::DamageSource, tool_stack::{adjust_damage, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{EntityState, ItemSlot, Nbt};
    let world = world();
    let helmet = &world.tools.iter().find(|(id, _)| id.path() == "plate_helmet").unwrap().1;
    let helmet_id = Identifier::parse("tconstruct:plate_helmet").unwrap();
    let armor_tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:plate_helmet"], &[("tconstruct:modifiable/armor", &["tconstruct:plate_helmet"])]).unwrap();
    let env = Environment { definition: Some(helmet), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&helmet_id), item_tags: Some(&armor_tags) };
    let (tags, problems) = DamageTags::load(&damage_tag_stacks());
    assert!(problems.is_empty(), "{problems:?}");
    let adjusted = |modifier: &str, level: i32, damage_type: &str, wearer: Option<&EntityState>, amount: f32| {
        let mut tool = ItemSlot::new(helmet_id.clone(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse(modifier).unwrap(), level).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        let damage_type = Identifier::parse(damage_type).unwrap();
        let source = DamageSource { damage_type: &damage_type, indirect: false, tags: &tags, attacker_in_tag: &|_| false, entities: Default::default() };
        adjust_damage(&tool.custom_data, &env, &source, &Entities { attacker: None, target: wearer }, amount)
    };
    let whole = facts(20.0);
    let hurt = facts(12.0);
    // warded: at full health each level takes a point off a hit, never below 1 and never raising it.
    assert_eq!(adjusted("tconstruct:warded", 2, "minecraft:mob_attack", Some(&whole), 5.0), 3.0);
    assert_eq!(adjusted("tconstruct:warded", 2, "minecraft:mob_attack", Some(&whole), 1.5), 1.0);
    assert_eq!(adjusted("tconstruct:warded", 2, "minecraft:mob_attack", Some(&whole), 0.5), 0.5);
    assert_eq!(adjusted("tconstruct:warded", 2, "minecraft:mob_attack", Some(&hurt), 5.0), 5.0, "not at full health");
    assert_eq!(adjusted("tconstruct:warded", 2, "minecraft:mob_attack", None, 5.0), 5.0, "a wearer that was not reported is not at full health");
    assert_eq!(adjusted("tconstruct:warded", 2, "minecraft:out_of_world", Some(&whole), 5.0), 5.0, "damage that cannot be protected against");
    // dragonheart: a hit is capped at the maximum health less two points a level (at least 1), for a wearer with 2 or more.
    assert_eq!(adjusted("tconstruct:dragonheart", 1, "minecraft:mob_attack", Some(&hurt), 30.0), 18.0);
    assert_eq!(adjusted("tconstruct:dragonheart", 1, "minecraft:mob_attack", Some(&hurt), 5.0), 5.0);
    let frail = EntityState { max_health: Some(1.0), ..facts(1.0) };
    assert_eq!(adjusted("tconstruct:dragonheart", 1, "minecraft:mob_attack", Some(&frail), 30.0), 30.0, "under 2 maximum health the module does not apply");
}

#[test]
fn a_range_condition_reads_a_variable_of_the_entity() {
    use crate::melee_damage::{state_in_tag, TargetCondition};
    use serde_json::json;
    let range = |min: Option<f32>, max: Option<f32>, interval: &str| {
        let mut condition = json!({"type": "tconstruct:variable_range", "variable": {"type": "tconstruct:attribute", "attribute": "minecraft:generic.max_health"}, "interval": interval});
        if let Some(min) = min { condition["min"] = json!(min); }
        if let Some(max) = max { condition["max"] = json!(max); }
        TargetCondition::decode_for("target", Some(&condition)).unwrap().expect("a max health range decodes")
    };
    let holds = |condition: &TargetCondition, max_health: Option<f32>| {
        let state = pluto_lib::EntityState { max_health, ..Default::default() };
        condition.matches(&|tag: &Identifier| state_in_tag(tag, None, Some(&state)))
    };
    let at_least_two = range(Some(2.0), None, "right_open");
    assert!(holds(&at_least_two, Some(2.0)) && holds(&at_least_two, Some(40.0)) && !holds(&at_least_two, Some(1.9)) && !holds(&at_least_two, None));
    let under_ten = range(None, Some(10.0), "left_open");
    assert!(holds(&under_ten, Some(10.0)) && holds(&under_ten, Some(-5.0)) && !holds(&under_ten, Some(10.5)));
    let open = range(Some(2.0), Some(4.0), "open");
    assert!(holds(&open, Some(3.0)) && !holds(&open, Some(2.0)) && !holds(&open, Some(4.0)));
    let light = json!({"type": "tconstruct:variable_range", "variable": "tconstruct:light", "min": 1.0});
    assert!(TargetCondition::decode_for("target", Some(&light)).unwrap().is_none(), "light is not reported");
}

#[test]
fn shipped_capacity_shields_soak_tool_wear_from_their_bar_and_every_filler_is_bound() {
    use crate::{modifier_entries::List, tool_stack::{rebuild_stats, Environment, TAG_MATERIALS, TAG_PERSISTENT, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt, NbtCompound};
    let world = world();
    let boots = &world.tools.iter().find(|(id, _)| id.path() == "plate_boots").unwrap().1;
    let env = Environment { definition: Some(boots), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let unbound = |name: &str| world.registry.get(&Identifier::parse(name).unwrap()).unwrap().unbound.iter().map(Identifier::to_string).collect::<Vec<_>>();
    assert!(unbound("tconstruct:barkskin").is_empty(), "the bar and its shield are both bound");
    assert!(unbound("tconstruct:stoneshield").is_empty(), "loot to capacity is bound to the loot of a mined block");
    assert!(unbound("tconstruct:frostshield").is_empty(), "damage to capacity is bound to the wearer's damage event");
    assert!(unbound("tconstruct:overwield").is_empty(), "overwield's melee, mining and launch drains are bound");
    assert!(unbound("tconstruct:war_charge").is_empty(), "the shot event reports the launcher's stack for an edit");
    assert!(unbound("tconstruct:necrotic").is_empty(), "lifesteal is bound");
    let tool = |modifier: &str, level: i32, stored: i32| {
        let mut tool = ItemSlot::new(Identifier::parse("tconstruct:plate_boots").unwrap(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse(modifier).unwrap(), level).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        let mut persistent = NbtCompound::new();
        persistent.insert(modifier.into(), Nbt::Int(stored));
        tool.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
        tool
    };
    let stored = |tool: &ItemSlot, modifier: &str| match &tool.custom_data[TAG_PERSISTENT] { Nbt::Compound(data) => data.get(modifier).map_or(0, |value| value.as_i64().unwrap() as i32), _ => 0 };
    for modifier in ["tconstruct:barkskin", "tconstruct:frostshield"] {
        let mut worn = tool(modifier, 1, 10);
        assert_eq!(crate::capacity::absorb(&mut worn.custom_data, &env, None, 4, &mut || 0.0), 0, "{modifier}");
        assert_eq!(stored(&worn, modifier), 6);
        assert_eq!(crate::capacity::absorb(&mut worn.custom_data, &env, None, 10, &mut || 0.0), 4);
        assert_eq!(stored(&worn, modifier), 0);
    }
    // The wear goes through the stack path too: the shield keeps the tool whole and the persistent data follows.
    let mut worn = tool("tconstruct:stoneshield", 1, 20);
    let outcome = crate::tool_slot::damage_slot(&mut worn, &env, 5, true, true, None, &mut || 0.0).unwrap();
    assert_eq!((outcome.applied, stored(&worn, "tconstruct:stoneshield")), (0, 15));
}

#[test]
fn shipped_mining_capacity_and_loot_to_capacity_edit_the_bars_of_the_tool_that_broke_the_block() {
    use crate::{modifier_entries::List, tool_stack::{loot_capacity, mining_capacity_edits, rebuild_stats, Environment, TAG_MATERIALS, TAG_PERSISTENT, TAG_STATS, TAG_UPGRADES}, weapon_effect::Usage};
    use pluto_lib::{ItemSlot, Nbt, NbtCompound, StackDataEdit};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let item = Identifier::parse("tconstruct:pickaxe").unwrap();
    let tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:pickaxe"], &[("tconstruct:modifiable/harvest", &["tconstruct:pickaxe"][..])]).unwrap();
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&item), item_tags: Some(&tags) };
    let tool = |modifier: &str, level: i32, persistent: &[(&str, i32)]| {
        let mut tool = ItemSlot::new(item.clone(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse(modifier).unwrap(), level).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        let mut data = NbtCompound::new();
        for (key, value) in persistent { data.insert((*key).into(), Nbt::Int(*value)); }
        tool.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(data));
        tool
    };
    let set = |key: &str, value: i32| StackDataEdit { path: vec![TAG_PERSISTENT.to_string(), key.to_string()], value: Some(Nbt::Int(value)) };
    // overwield spends one overslime a level as the dig starts (it runs before the break), and nothing after it.
    let mut overwield = tool("tconstruct:overwield", 2, &[("tconstruct:overslime", 50)]);
    let Some(Nbt::Compound(stats)) = overwield.custom_data.get_mut(TAG_STATS) else { panic!("rebuilt stats") };
    stats.insert("tconstruct:overslime".into(), Nbt::Float(100.0));
    assert_eq!(mining_capacity_edits(&overwield.custom_data, &env, true), vec![set("tconstruct:overslime", 48)]);
    assert!(mining_capacity_edits(&overwield.custom_data, &env, false).is_empty());
    let mut empty = overwield.clone();
    let Some(Nbt::Compound(persistent)) = empty.custom_data.get_mut(TAG_PERSISTENT) else { panic!("persistent data") };
    persistent.insert("tconstruct:overslime".into(), Nbt::Int(1));
    assert_eq!(mining_capacity_edits(&empty.custom_data, &env, true), vec![StackDataEdit { path: vec![TAG_PERSISTENT.to_string(), "tconstruct:overslime".to_string()], value: None }], "a drained bar is removed");
    // war charge gains one after a break, only while the tool holds a crossbow ammo key.
    let charged = tool("tconstruct:war_charge", 1, &[("tconstruct:crossbow_ammo", 1), ("tconstruct:war_charge", 4)]);
    assert_eq!(mining_capacity_edits(&charged.custom_data, &env, false), vec![set("tconstruct:war_charge", 5)]);
    assert!(mining_capacity_edits(&charged.custom_data, &env, true).is_empty());
    assert!(mining_capacity_edits(&tool("tconstruct:war_charge", 1, &[("tconstruct:war_charge", 4)]).custom_data, &env, false).is_empty(), "no ammo key, no gain");
    // launch capacity: war charge has no grant, which empties its bar on every shot; overwield spends overslime only for ranged tools.
    use crate::tool_stack::launch_capacity_edits;
    assert_eq!(launch_capacity_edits(&charged.custom_data, &env), vec![StackDataEdit { path: vec![TAG_PERSISTENT.to_string(), "tconstruct:war_charge".to_string()], value: None }]);
    assert!(launch_capacity_edits(&tool("tconstruct:war_charge", 1, &[]).custom_data, &env).is_empty(), "an empty bar stays empty");
    assert!(launch_capacity_edits(&overwield.custom_data, &env).is_empty(), "a pickaxe is no ranged tool");
    let ranged = pluto_lib::fixtures::registry_with_tags(&["tconstruct:pickaxe"], &[("tconstruct:modifiable/ranged", &["tconstruct:pickaxe"][..])]).unwrap();
    let ranged_env = Environment { item_tags: Some(&ranged), ..Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry, item_tags_loaded: true, item: Some(&item), item_tags: None } };
    assert_eq!(launch_capacity_edits(&overwield.custom_data, &ranged_env), vec![set("tconstruct:overslime", 48)], "one overslime a level for each shot");
    // lifesteal: necrotic heals 5% of the damage a level, except from a mob of the necrotic blacklist.
    let necrotic = tool("tconstruct:necrotic", 2, &[]);
    let heal = |blacklisted: bool| crate::tool_stack::lifesteal_heal(&necrotic.custom_data, &env, 10.0, blacklisted, &|_| true, &|_| true);
    assert!((heal(false) - 1.0).abs() < 1e-5 && heal(true) == 0.0, "{} {}", heal(false), heal(true));
    assert_eq!(crate::tool_stack::lifesteal_heal(&overwield.custom_data, &env, 10.0, false, &|_| true, &|_| true), 0.0, "a tool without the module heals nothing");
    // stoneshield eats the stone of the loot into its bar: 40% a stone at level 2, three points a stone.
    let shield = tool("tconstruct:stoneshield", 2, &[]);
    let in_tag = |item: &Identifier, tag: &Identifier| tag.to_string() == "tconstruct:stoneshields" && item.path() == "cobblestone";
    let loot = || vec![ItemSlot::new(Identifier::parse("minecraft:cobblestone").unwrap(), 3), ItemSlot::new(Identifier::parse("minecraft:diamond").unwrap(), 1)];
    let mut rolls = [0.1, 0.9, 0.3].into_iter();
    let mut dropped = loot();
    let edits = loot_capacity(&shield.custom_data, &env, &mut dropped, &in_tag, &mut || rolls.next().unwrap());
    assert_eq!(edits, vec![set("tconstruct:stoneshield", 6)], "two of the three stones are eaten");
    assert_eq!(dropped.iter().map(|slot| (slot.item.path().to_string(), slot.count)).collect::<Vec<_>>(), [("cobblestone".to_string(), 1), ("diamond".to_string(), 1)]);
    let mut all = loot();
    assert_eq!(loot_capacity(&shield.custom_data, &env, &mut all, &in_tag, &mut || 0.0), vec![set("tconstruct:stoneshield", 9)]);
    assert_eq!(all.len(), 1, "only the diamond is left");
    let mut untouched = loot();
    assert!(loot_capacity(&tool("tconstruct:sharpness", 1, &[]).custom_data, &env, &mut untouched, &in_tag, &mut || 0.0).is_empty());
    assert_eq!(untouched, loot(), "a tool without the module leaves the loot alone");
    // The block-break half of the tool usage effects: decay names no such hook, magnetic hears every one.
    assert!(crate::tool_stack::usage_effects(&tool("tconstruct:decay", 1, &[]).custom_data, &env, Usage::BlockBreak, &mut || 0.0).is_empty());
    let magnetic = crate::tool_stack::usage_effects(&tool("tconstruct:magnetic", 1, &[]).custom_data, &env, Usage::BlockBreak, &mut || 0.99);
    assert_eq!(magnetic.iter().map(|grant| (grant.effect.to_string(), grant.duration)).collect::<Vec<_>>(), [("tconstruct:magnetic".to_string(), 40)]);
}

/// A rebuilt tool of a shipped definition carrying `upgrades`.
fn upgraded(world: &World, tool: &str, upgrades: &[(&str, i32)]) -> pluto_lib::ItemSlot {
    use crate::{modifier_entries::List, tool_stack::{rebuild_stats, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let env = env_of(world, tool);
    let mut stack = ItemSlot::new(Identifier::new("tconstruct", tool).unwrap(), 1);
    if env.has_materials { stack.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec())); }
    let mut list = List::default();
    for (modifier, level) in upgrades { list = list.with_modifier(Identifier::parse(modifier).unwrap(), *level).unwrap(); }
    stack.custom_data.insert(TAG_UPGRADES.into(), list.to_nbt());
    assert!(rebuild_stats(&mut stack.custom_data, &env).unwrap());
    stack
}
fn env_of<'a>(world: &'a World, tool: &str) -> crate::tool_stack::Environment<'a> {
    let definition = &world.tools.iter().find(|(id, _)| id.path() == tool).unwrap().1;
    crate::tool_stack::Environment { definition: Some(definition), has_materials: !definition.tool_parts().is_empty(), materials: &world.materials, modifiers: &world.registry, item_tags_loaded: true, item: None, item_tags: None }
}

#[test]
fn shipped_tanned_limits_the_wear_of_ordinary_use_to_one_point() {
    let world = world();
    let plain = upgraded(&world, "pickaxe", &[]);
    let tanned = upgraded(&world, "pickaxe", &[("tconstruct:tanned", 1)]);
    let env = env_of(&world, "pickaxe");
    let wear = |stack: &pluto_lib::ItemSlot, amount| crate::tool_stack::reduce_damage(&stack.custom_data, &env, None, amount, &mut || panic!("a range rolls nothing"));
    assert_eq!(wear(&plain, 7), 7);
    assert_eq!(wear(&tanned, 7), 1, "a range with max 1 clamps ordinary use");
    assert_eq!(wear(&tanned, 1), 1);
    // A cause that is a modifier outside the bypass tag is clamped too; with no modifier tags loaded none is in a tag.
    let cause = Identifier::parse("tconstruct:unknown").unwrap();
    assert_eq!(crate::tool_stack::reduce_damage(&tanned.custom_data, &env, Some(&cause), 4, &mut || 0.0), 1);
}

#[test]
fn shipped_overlord_copies_a_tenth_of_the_final_durability_into_overslime_per_level() {
    use pluto_lib::Nbt;
    let world = world();
    let stat = |stack: &pluto_lib::ItemSlot, name: &str| match stack.custom_data.get(crate::tool_stack::TAG_STATS) {
        Some(Nbt::Compound(stats)) => stats.get(&format!("tconstruct:{name}")).and_then(Nbt::as_f64).map(|value| value as f32),
        _ => None,
    };
    assert_eq!(stat(&upgraded(&world, "pickaxe", &[]), "overslime"), None);
    let one = upgraded(&world, "pickaxe", &[("tconstruct:overlord", 1)]);
    let three = upgraded(&world, "pickaxe", &[("tconstruct:overlord", 3)]);
    // The copy runs after the durability penalty (-15% per level), so it sees the reduced durability.
    for (stack, level) in [(&one, 1.0_f32), (&three, 3.0)] {
        let durability = stat(stack, "durability").unwrap();
        let overslime = stat(stack, "overslime").unwrap();
        assert!((overslime - durability * 0.1 * level).abs() < 0.01, "level {level}: {overslime} of {durability}");
    }
    let plain = stat(&upgraded(&world, "pickaxe", &[]), "durability").unwrap();
    assert!(stat(&one, "durability").unwrap() < plain, "the durability penalty applies");
}

#[test]
fn shipped_soulspeed_is_a_soul_speed_enchantment_and_trim_adds_a_slot_only_without_a_trim_hook() {
    use crate::tool_stack::{enchantments, TAG_VOLATILE};
    use pluto_lib::Nbt;
    let world = world();
    let soul = upgraded(&world, "pickaxe", &[("tconstruct:soulspeed", 1)]);
    assert_eq!(enchantments(&soul.custom_data, &env_of(&world, "pickaxe")).get("minecraft:soul_speed"), Some(&1));
    let defense = |stack: &pluto_lib::ItemSlot| match stack.custom_data.get(TAG_VOLATILE) {
        Some(Nbt::Compound(data)) => data.get("defense").and_then(Nbt::as_i64).unwrap_or(0),
        _ => 0,
    };
    assert_eq!(defense(&upgraded(&world, "pickaxe", &[("tconstruct:trim", 1)])), 1, "a tool with no trim trait hook gets a defense slot");
    assert_eq!(defense(&upgraded(&world, "slime_helmet", &[("tconstruct:trim", 1)])), 0, "a tool whose definition binds the trim trait hook does not");
}

#[test]
fn shipped_attributes_on_forge_names_publish_the_vanilla_attribute_and_the_rest_stay_unbound() {
    use crate::tool_stack::attribute_entries;
    use pluto_lib::Nbt;
    let world = world();
    let amounts = |modifier: &str, level: i32, slot: &str| -> Vec<(String, f64)> {
        let stack = upgraded(&world, "pickaxe", &[(modifier, level)]);
        attribute_entries(&stack.custom_data, &env_of(&world, "pickaxe"), &[slot]).iter().map(|entry| {
            let (Some(Nbt::String(kind)), Some(Nbt::Double(amount))) = (entry.get("type"), entry.get("amount")) else { panic!("an attribute entry") };
            (kind.clone(), *amount)
        }).collect()
    };
    let reach = amounts("tconstruct:reach", 2, "mainhand");
    assert!(reach.contains(&("minecraft:block_interaction_range".into(), 2.0)) && reach.contains(&("minecraft:entity_interaction_range".into(), 2.0)), "{reach:?}");
    assert_eq!(amounts("tconstruct:step_up", 2, "feet"), vec![("minecraft:step_height".to_string(), 1.0)]);
    let sky = amounts("tconstruct:skyfall", 1, "mainhand");
    let gravity = sky.iter().find(|(kind, _)| kind == "minecraft:gravity").expect("gravity").1;
    assert!((gravity - -0.15).abs() < 1e-6, "{sky:?}");
    assert!(sky.contains(&("minecraft:safe_fall_distance".into(), 1.0)));
    // Attributes the host has no counterpart for are not published; the modifier keeps them unbound.
    assert!(amounts("tconstruct:double_jump", 1, "feet").is_empty());
    let mut registry = Registry::default();
    registry.reload(&files("modifiers"), &conditions());
    let unbound = |name: &str| registry.get(&Identifier::parse(name).unwrap()).unwrap().unbound.iter().map(ToString::to_string).collect::<Vec<_>>();
    assert!(unbound("tconstruct:reach").is_empty() && unbound("tconstruct:step_up").is_empty() && unbound("tconstruct:skyfall").is_empty());
    assert_eq!(unbound("tconstruct:turtles_grace"), vec!["tconstruct:attribute"], "forge:swim_speed has no vanilla attribute");
    assert_eq!(unbound("tconstruct:double_jump"), vec!["tconstruct:attribute"]);
    // The modules that need host data this port does not receive stay unbound.
    for (modifier, kind) in [("tconstruct:gold_guard", "golden_attribute"), ("tconstruct:lightspeed", "lightspeed_attribute"), ("tconstruct:tank", "stat_boost")] {
        assert!(unbound(modifier).contains(&format!("tconstruct:{kind}")), "{modifier} keeps {kind} unbound");
    }
    // Experienced's volatile float is bound now; only its experience multiplier attribute has no counterpart.
    assert_eq!(unbound("tconstruct:experienced"), vec!["tconstruct:attribute"]);
    assert!(unbound("tconstruct:tanned").is_empty() && unbound("tconstruct:overlord").is_empty() && unbound("tconstruct:soulspeed").is_empty());
}

#[test]
fn shipped_fire_protection_takes_back_the_vanilla_points_its_enchantment_gave() {
    use crate::{damage_tags::DamageTags, modifier_entries::List, protection::{damage_after_protection, host_enchantment_points, DamageSource}, tool_stack::{enchantments, protection, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{EntityState, ItemSlot, Nbt};
    let world = world();
    let helmet = &world.tools.iter().find(|(id, _)| id.path() == "plate_helmet").unwrap().1;
    let helmet_id = Identifier::parse("tconstruct:plate_helmet").unwrap();
    let armor_tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:plate_helmet"], &[("tconstruct:modifiable/armor", &["tconstruct:plate_helmet"])]).unwrap();
    let env = Environment { definition: Some(helmet), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&helmet_id), item_tags: Some(&armor_tags) };
    let (tags, problems) = DamageTags::load(&damage_tag_stacks());
    assert!(problems.is_empty(), "{problems:?}");
    let id = |text: &str| Identifier::parse(text).unwrap();
    // The helmet, the points the game's enchantment protection counted for it, and the running protection the modifiers leave.
    let run = |modifier: &str, level: i32, damage_type: &str, attacker: Option<&EntityState>| {
        let mut tool = ItemSlot::new(helmet_id.clone(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(id(modifier), level).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        let damage_type = id(damage_type);
        let in_tag = |tag: &Identifier| tag.namespace() == crate::melee_damage::STATE_NAMESPACE && crate::melee_damage::state_in_tag(tag, attacker, None);
        let source = DamageSource { damage_type: &damage_type, indirect: false, tags: &tags, attacker_in_tag: &in_tag, entities: Default::default() };
        let vanilla = host_enchantment_points([&enchantments(&tool.custom_data, &env)], &source);
        (vanilla, protection(&tool.custom_data, &env, &source, vanilla))
    };
    // Fire protection I is 2 vanilla points against fire, which come off again before its own 3 are added.
    assert_eq!(run("tconstruct:fire_protection", 1, "minecraft:lava", None), (2.0, 3.0));
    // Iron plating brings its own projectile and melee protection, so the unrelated baseline is measured with an unrelated modifier.
    let base = |damage_type: &str| run("tconstruct:sharpness", 1, damage_type, None).1;
    assert_eq!(run("tconstruct:fire_protection", 1, "minecraft:arrow", None), (0.0, base("minecraft:arrow")), "no fire, no points either way");
    assert_eq!(run("tconstruct:fire_protection", 1, "minecraft:sonic_boom", None), (0.0, base("minecraft:sonic_boom")), "damage that bypasses enchantments counts none");
    assert_eq!(run("tconstruct:blast_protection", 1, "minecraft:explosion", None).0, 2.0, "blast protection is 2 points a level against explosions");
    let (vanilla, total) = run("tconstruct:fireborn", 1, "minecraft:lava", None);
    assert_eq!((vanilla, total), (14.0, 0.0), "fireborn's level 7 is all taken back");
    // The event's damage already had the vanilla points: they are undone, then the modifiers' protection applies.
    assert!((damage_after_protection(10.0, 2.0, 3.0, 20.0) - 10.0 / 0.92 * 0.88).abs() < 1e-4);
    assert!((damage_after_protection(8.0, 14.0, 0.0, 20.0) - 8.0 / (1.0 - 14.0 / 25.0)).abs() < 1e-4, "ignoring the whole enchantment gives the damage back");
    // Scorch protection protects against a fire-immune attacker only.
    let immune = EntityState { kind: "minecraft:blaze".into(), fire_immune: true, ..EntityState::default() };
    let normal = EntityState { kind: "minecraft:husk".into(), ..EntityState::default() };
    assert_eq!(run("tconstruct:scorch_protection", 2, "minecraft:mob_attack", Some(&immune)), (0.0, base("minecraft:mob_attack") + 2.5));
    assert_eq!(run("tconstruct:scorch_protection", 2, "minecraft:mob_attack", Some(&normal)), (0.0, base("minecraft:mob_attack")));
    assert_eq!(run("tconstruct:scorch_protection", 2, "minecraft:mob_attack", None), (0.0, base("minecraft:mob_attack")), "an unknown attacker is not fire immune");
}

#[test]
fn shipped_revenge_names_the_effect_that_unequipping_takes_off() {
    use crate::{modifier_entries::List, tool_stack::{effects_cleared_on_unequip, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let helmet = &world.tools.iter().find(|(id, _)| id.path() == "plate_helmet").unwrap().1;
    let helmet_id = Identifier::parse("tconstruct:plate_helmet").unwrap();
    let armor_tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:plate_helmet"], &[("tconstruct:modifiable/armor", &["tconstruct:plate_helmet"])]).unwrap();
    let env = Environment { definition: Some(helmet), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&helmet_id), item_tags: Some(&armor_tags) };
    let id = |text: &str| Identifier::parse(text).unwrap();
    let cleared = |modifier: &str| {
        let mut tool = ItemSlot::new(helmet_id.clone(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(id(modifier), 1).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        effects_cleared_on_unequip(&tool.custom_data, &env)
    };
    assert_eq!(cleared("tconstruct:revenge"), vec![id("minecraft:strength")]);
    assert!(cleared("tconstruct:fire_protection").is_empty());
}

#[test]
fn shipped_frostshield_turns_freezing_damage_into_bar_and_overwield_drains_overslime_on_a_hit() {
    use crate::{damage_tags::DamageTags, modifier_entries::List, overslime, protection::DamageSource, tool_stack::{damage_capacity, melee_capacity_edits, rebuild_stats, Environment, TAG_MATERIALS, TAG_PERSISTENT, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt, NbtCompound, StackDataEdit};
    let mut world = world();
    for modifier in overslime::code_modifiers().unwrap() { world.registry.register_static(modifier).unwrap(); }
    let id = |text: &str| Identifier::parse(text).unwrap();
    let (tags, problems) = DamageTags::load(&damage_tag_stacks());
    assert!(problems.is_empty(), "{problems:?}");
    let persist = |modifier: &str| vec![TAG_PERSISTENT.to_string(), modifier.to_string()];

    // Frostshield on boots: freezing damage fills the bar by its amount and the wearer is spared that much.
    let boots = &world.tools.iter().find(|(tool, _)| tool.path() == "plate_boots").unwrap().1;
    let env = Environment { definition: Some(boots), has_materials: true, materials: &world.materials, modifiers: &world.registry, item_tags_loaded: true, item: None, item_tags: None };
    let frost = |stored: i32| {
        let mut tool = ItemSlot::new(id("tconstruct:plate_boots"), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(id("tconstruct:frostshield"), 1).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        if stored > 0 {
            let mut persistent = NbtCompound::new();
            persistent.insert("tconstruct:frostshield".into(), Nbt::Int(stored));
            tool.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
        }
        tool
    };
    let taken = |tool: &ItemSlot, damage_type: &str, amount: f32| {
        let damage_type = id(damage_type);
        damage_capacity(&tool.custom_data, &env, &DamageSource { damage_type: &damage_type, indirect: false, tags: &tags, attacker_in_tag: &|_| false, entities: Default::default() }, amount)
    };
    let set = |edits: &[StackDataEdit], value: i32| edits == [StackDataEdit { path: persist("tconstruct:frostshield"), value: Some(Nbt::Int(value)) }];
    let (left, edits) = taken(&frost(0), "minecraft:freeze", 3.0);
    assert!(left == 0.0 && set(&edits, 3), "3 freezing damage fill 3 of an empty bar: {left} {edits:?}");
    let (left, edits) = taken(&frost(10), "minecraft:freeze", 2.5);
    assert!(left == -0.5 && set(&edits, 13), "the amount is rounded up to whole bar points and the surplus comes off the damage as the original does: {left} {edits:?}");
    let (left, edits) = taken(&frost(0), "minecraft:mob_attack", 3.0);
    assert!(left == 3.0 && edits.is_empty(), "only the freezing source fills the bar");
    let capacity = crate::capacity::Bar::decode(serde_json::json!({"each_level":100,"multiplier":"tconstruct:durability"}).as_object().unwrap()).unwrap().unwrap().capacity(&frost(0).custom_data, &crate::modifier_entries::Entry::new(id("tconstruct:frostshield"), 1));
    assert!(capacity > 3);
    let (left, edits) = taken(&frost(capacity - 1), "minecraft:freeze", 3.0);
    assert!(left == 2.0 && set(&edits, capacity), "a nearly full bar takes one point, capped, and the wearer the rest: {left} {edits:?}");
    let (left, edits) = taken(&frost(capacity), "minecraft:freeze", 3.0);
    assert!(left == 3.0 && edits.is_empty(), "a full bar leaves the damage to the wearer");

    // Overwield on a pickaxe: each hit drains overslime by the level; an emptied bar loses its key.
    let pickaxe = &world.tools.iter().find(|(tool, _)| tool.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry, item_tags_loaded: true, item: None, item_tags: None };
    let wield = |level: i32, stored: i32| {
        let mut tool = ItemSlot::new(id("tconstruct:pickaxe"), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(overslime::id(), 1).unwrap().with_modifier(id("tconstruct:overwield"), level).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        if stored > 0 {
            let mut persistent = NbtCompound::new();
            persistent.insert("tconstruct:overslime".into(), Nbt::Int(stored));
            tool.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
        }
        tool
    };
    let hit = |tool: &ItemSlot| melee_capacity_edits(&tool.custom_data, &env, &|_| false, &|_| false);
    let drained = |edits: &[StackDataEdit], value: Option<i32>| edits == [StackDataEdit { path: persist("tconstruct:overslime"), value: value.map(Nbt::Int) }];
    assert!(drained(&hit(&wield(2, 30)), Some(28)), "level 2 drains 2 a hit");
    assert!(drained(&hit(&wield(1, 1)), None), "a drained bar loses its key");
    assert!(hit(&wield(1, 0)).is_empty(), "an empty bar has nothing to drain");
}

/// An edible pickaxe through the shipped modifiers: the rebuilt stats, the meal the conditional stats make of them, and what eating costs the tool.
fn edible_pickaxe<R>(upgrades: &[(&str, i32)], charge_tagged: bool, run: impl FnOnce(&crate::tool_stack::Environment, &pluto_lib::ItemSlot) -> R) -> R {
    use crate::{modifier_entries::List, tool_stack::{rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let id = Identifier::parse("tconstruct:pickaxe").unwrap();
    let members: &[&str] = if charge_tagged { &["tconstruct:pickaxe"] } else { &[] };
    let tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:pickaxe"], &[("tconstruct:modifiable/interactable/charge", members)]).unwrap();
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&id), item_tags: Some(&tags) };
    let mut slot = ItemSlot::new(id.clone(), 1);
    slot.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
    let mut list = List::default();
    for (modifier, level) in upgrades { list = list.with_modifier(Identifier::parse(modifier).unwrap(), *level).unwrap(); }
    slot.custom_data.insert(TAG_UPGRADES.into(), list.to_nbt());
    assert!(rebuild_stats(&mut slot.custom_data, &env).unwrap());
    run(&env, &slot)
}

#[test]
fn the_shipped_edible_family_binds_all_but_the_representative_item() {
    let mut registry = Registry::default();
    registry.reload(&files("modifiers"), &conditions());
    let unbound = |name: &str| registry.get(&Identifier::parse(name).unwrap()).unwrap().unbound.iter().map(ToString::to_string).collect::<Vec<_>>();
    assert!(unbound("tconstruct:edible").is_empty(), "the edible module and its tooltip trait are bound");
    for name in ["tasty", "savory", "scrumptious"] {
        assert_eq!(unbound(&format!("tconstruct:{name}")), vec!["tconstruct:edible_representative_item"], "{name} keeps only the particles-and-Diet module unbound");
    }
}

#[test]
fn an_eaten_tool_feeds_the_rounded_stats_and_runs_each_modifiers_edible_effects_in_order() {
    use crate::{edible::Effect, edible_hook::meal, tool_stack::{edible, stat}, tool_stats::Stat};
    // tasty II: hunger +1 a level, saturation +0.6, counter chance +0.15 a level, wearing 5 + 5 a level.
    edible_pickaxe(&[("tconstruct:tasty", 2)], true, |env, tool| {
        let nbt = &tool.custom_data;
        assert_eq!((stat(nbt, Stat::Hunger), stat(nbt, Stat::EatDuration)), (2.0, 16.0), "the eat duration keeps its default of 16 ticks");
        assert!((stat(nbt, Stat::Saturation) - 0.6).abs() < 1e-6 && (stat(nbt, Stat::EdibleCounterChance) - 0.3).abs() < 1e-6);
        let eaten = meal(nbt, env).expect("a tasty tool is a meal");
        assert_eq!((eaten.nutrition, eaten.effects), (2, vec![Effect::ConsumeDurability(15)]));
        assert!((eaten.saturation - 0.6).abs() < 1e-6);
    });
    // savory I: wears 4 + 4, then cures one effect; the representative item is unbound and adds nothing.
    edible_pickaxe(&[("tconstruct:savory", 1)], true, |env, tool| {
        assert_eq!(edible(&tool.custom_data, env), Some(vec![Effect::ConsumeDurability(8), Effect::CureRandomEffect]));
        assert!((stat(&tool.custom_data, Stat::Saturation) - 0.4).abs() < 1e-6);
    });
    // scrumptious I: wears 5 + 3, then removes poison.
    edible_pickaxe(&[("tconstruct:scrumptious", 1)], true, |env, tool| {
        let poison = Identifier::parse("minecraft:poison").unwrap();
        assert_eq!(edible(&tool.custom_data, env), Some(vec![Effect::ConsumeDurability(8), Effect::RemoveEffect(poison)]));
    });
    // Two different modifiers add their hunger and run their effects in modifier order.
    edible_pickaxe(&[("tconstruct:tasty", 1), ("tconstruct:scrumptious", 1)], true, |env, tool| {
        let eaten = meal(&tool.custom_data, env).unwrap();
        assert_eq!(eaten.nutrition, 2);
        assert_eq!(eaten.effects.len(), 3, "tasty's wear, scrumptious's wear and the poison cure: {:?}", eaten.effects);
    });
    // A plain tool is no meal.
    edible_pickaxe(&[("tconstruct:sharpness", 1)], true, |env, tool| {
        assert_eq!(edible(&tool.custom_data, env), None);
        assert_eq!(meal(&tool.custom_data, env), None);
    });
}

#[test]
fn eating_wears_the_tool_but_not_for_a_creative_eater_and_a_tool_outside_the_valid_tags_gets_no_hunger() {
    use crate::{edible_hook::{meal, pay}, tool_slot, tool_stack::{damage, stat}, tool_stats::Stat};
    edible_pickaxe(&[("tconstruct:tasty", 1)], true, |env, tool| {
        let eaten = meal(&tool.custom_data, env).unwrap();
        let before = damage(&tool.custom_data);
        let worn = pay(&eaten, tool, env, false);
        assert_eq!(damage(&worn.custom_data) - before, 10, "tasty I wears 5 + 5");
        assert_eq!(worn.damage, tool.damage + 10, "the slot's bar follows the tool's damage");
        assert_eq!(pay(&eaten, tool, env, true), *tool, "a creative eater pays nothing");
        // eating cannot wear past what is left: the tool breaks instead
        let mut nearly_gone = tool.clone();
        tool_slot::direct_damage_slot(&mut nearly_gone, env, tool_slot::remaining(tool) - 3, true).unwrap();
        let broken = pay(&eaten, &nearly_gone, env, false);
        assert!(tool_slot::broken(&broken), "the last 3 points of durability are all eating can take");
    });
    // Hunger applies only to the tools EdibleModule.VALID_TOOLS names (charging tools and armor).
    edible_pickaxe(&[("tconstruct:tasty", 1)], false, |env, tool| {
        assert_eq!(stat(&tool.custom_data, Stat::Hunger), 0.0);
        assert_eq!(crate::edible_hook::meal(&tool.custom_data, env), None, "a tool with no hunger rounds to no meal");
    });
}

#[test]
fn the_items_eaten_by_a_right_click_are_the_tools_in_the_shipped_charge_tag() {
    fn members(tag: &str, found: &mut std::collections::BTreeSet<String>) {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content/data/tconstruct/tags/item").join(format!("{tag}.json"));
        let value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|_| panic!("tag file {path:?}"))).unwrap();
        for entry in value["values"].as_array().unwrap() {
            let id = entry.as_str().or_else(|| entry["id"].as_str()).unwrap();
            match id.strip_prefix("#tconstruct:") { Some(nested) => members(nested, found), None => { found.insert(id.to_owned()); } }
        }
    }
    let mut charge = std::collections::BTreeSet::new();
    members("modifiable/interactable/charge", &mut charge);
    // The original's other-mod minotaur axe is not a tool here.
    let tools: std::collections::BTreeSet<String> = crate::tool_parts::tool_items().map(|id| id.to_string()).collect();
    let charge: std::collections::BTreeSet<String> = charge.into_iter().filter(|id| tools.contains(id)).collect();
    let used: std::collections::BTreeSet<String> = crate::edible_hook::USE_ITEMS.iter().map(|path| format!("tconstruct:{path}")).collect();
    assert_eq!(used, charge, "the edible use covers exactly the charging tools, and no armor");
}

/// A plate helmet carrying `modifiers`, with the armor tags a protection condition asks about, ready for the wearer tests.
fn armor_helmet(world: &World, modifiers: &[(&str, i32)]) -> (pluto_lib::ItemSlot, pluto_lib::ItemSlot) {
    use crate::{modifier_entries::List, tool_stack::{rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::Nbt;
    let helmet = &world.tools.iter().find(|(id, _)| id.path() == "plate_helmet").unwrap().1;
    let helmet_id = Identifier::parse("tconstruct:plate_helmet").unwrap();
    let armor_tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:plate_helmet"], &[("tconstruct:modifiable/armor", &["tconstruct:plate_helmet"]), ("tconstruct:modifiable/armor/helmets", &["tconstruct:plate_helmet"])]).unwrap();
    let env = Environment { definition: Some(helmet), has_materials: true, materials: &world.materials, modifiers: &world.registry, item_tags_loaded: true, item: Some(&helmet_id), item_tags: Some(&armor_tags) };
    let mut tool = pluto_lib::ItemSlot::new(helmet_id.clone(), 1);
    tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
    let mut list = List::default();
    for (name, level) in modifiers { list = list.with_modifier(Identifier::parse(&format!("tconstruct:{name}")).unwrap(), *level).unwrap(); }
    tool.custom_data.insert(TAG_UPGRADES.into(), list.to_nbt());
    assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
    (tool.clone(), tool)
}

/// The plate helmet's definition, the armor item tags and the environment the wearer tests run the modifiers in.
macro_rules! helmet_env {
    ($world:ident, $env:ident) => {
        let helmet = &$world.tools.iter().find(|(id, _)| id.path() == "plate_helmet").unwrap().1;
        let helmet_id = Identifier::parse("tconstruct:plate_helmet").unwrap();
        let armor_tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:plate_helmet"], &[("tconstruct:modifiable/armor", &["tconstruct:plate_helmet"]), ("tconstruct:modifiable/armor/helmets", &["tconstruct:plate_helmet"])]).unwrap();
        let $env = crate::tool_stack::Environment { definition: Some(helmet), has_materials: true, materials: &$world.materials, modifiers: &$world.registry, item_tags_loaded: true, item: Some(&helmet_id), item_tags: Some(&armor_tags) };
    };
}

#[test]
fn shipped_armor_protection_reads_the_wearers_health_light_height_and_stance() {
    use crate::{damage_tags::DamageTags, protection::DamageSource, tool_stack::protection};
    use pluto_lib::EntityState;
    let world = world();
    helmet_env!(world, env);
    let (tags, problems) = DamageTags::load(&damage_tag_stacks());
    assert!(problems.is_empty(), "{problems:?}");
    let damage_type = Identifier::parse("minecraft:mob_attack").unwrap();
    // The protection the modifier gives over what iron plate gives by itself, for a wearer in the state `wearer` (None: no wearer reported).
    let gain = |modifier: &str, level: i32, wearer: Option<&EntityState>| {
        let of = |name: &str, level: i32| {
            let (tool, _) = armor_helmet(&world, &[(name, level)]);
            let source = DamageSource { damage_type: &damage_type, indirect: false, tags: &tags, attacker_in_tag: &|_| false, entities: crate::entity_facts::Entities { attacker: None, target: wearer } };
            protection(&tool.custom_data, &env, &source, 0.0)
        };
        of(modifier, level) - of("sharpness", 1)
    };
    let close = |value: f32, wanted: f32| assert!((value - wanted).abs() < 1e-4, "{value} is not {wanted}");
    // Vital: more protection the lower the health, from 11 points down: (11 - 4) / 10 of 2.5 a level.
    let health = |health: f32| EntityState { health: Some(health), max_health: Some(20.0), ..EntityState::default() };
    close(gain("vital_protection", 1, Some(&health(4.0))), 1.75);
    close(gain("vital_protection", 1, Some(&health(20.0))), 0.0);
    close(gain("vital_protection", 1, None), 2.5);
    // Loyal: protection for each empty armor slot, (1 - coverage) * 10.
    close(gain("loyal", 1, Some(&EntityState { armor_coverage: Some(0.25), ..EntityState::default() })), 7.5);
    close(gain("loyal", 1, Some(&EntityState { armor_coverage: Some(1.0), ..EntityState::default() })), 0.0);
    // Nightmare: 0.125 a level for each point of light below 10, the sky light less what the night takes, or the block light if that is brighter.
    let lit = |sky: u8, darken: u8, block: u8| EntityState { sky_light: Some(sky), sky_darken: darken, block_light: Some(block), ..EntityState::default() };
    close(gain("nightmare_protection", 1, Some(&lit(15, 11, 2))), 0.75);
    close(gain("nightmare_protection", 1, Some(&lit(15, 11, 8))), 0.25);
    close(gain("nightmare_protection", 1, Some(&lit(15, 0, 0))), 0.0);
    // Stance: dragonborn in the air, shulking while crouching.
    close(gain("dragonborn", 2, Some(&EntityState { on_ground: false, ..EntityState::default() })), 4.0);
    close(gain("dragonborn", 2, Some(&EntityState { on_ground: true, ..EntityState::default() })), 0.0);
    close(gain("dragonborn", 2, Some(&EntityState { on_ground: false, in_water: true, ..EntityState::default() })), 0.0);
    close(gain("shulking", 2, Some(&EntityState { crouching: true, ..EntityState::default() })), 5.0);
    close(gain("shulking", 2, Some(&EntityState::default())), 0.0);
    close(gain("shulking", 2, None), 0.0);
    // A turtle shell's helmet protects while the eyes are in water, 2.5 a level; the feet in water protect the legs and boots instead.
    close(gain("turtle_shell", 2, Some(&EntityState { eyes_in_water: true, ..EntityState::default() })), 5.0);
    close(gain("turtle_shell", 2, Some(&EntityState { in_water: true, ..EntityState::default() })), 0.0);
    // Temperature: flamestance adds the biome temperature less 0.75.
    close(gain("flamestance", 1, Some(&EntityState { biome_temperature: Some(2.0), ..EntityState::default() })), 1.25);
    close(gain("flamestance", 1, Some(&EntityState { biome_temperature: Some(0.0), ..EntityState::default() })), -0.75);
    // Depth: 1.25 a level, grown linearly below y 64 (half the maximum bonus of 1 at y 32), a penalty above the neutral range.
    close(gain("depth_protection", 2, Some(&EntityState { y: Some(32.0), ..EntityState::default() })), 1.25);
    close(gain("depth_protection", 2, Some(&EntityState { y: Some(70.0), ..EntityState::default() })), 0.0);
    close(gain("depth_protection", 2, Some(&EntityState { y: Some(112.0), ..EntityState::default() })), -0.625);
    close(gain("depth_protection", 2, Some(&EntityState { y: Some(200.0), ..EntityState::default() })), -2.5);
}

#[test]
fn shipped_overshield_turns_overslime_into_protection_and_the_edit_takes_it_off_the_piece() {
    use crate::{damage_tags::DamageTags, protection::DamageSource, tool_stack::{protection_with_edits, TAG_PERSISTENT}};
    use pluto_lib::{Nbt, StackDataEdit};
    let mut world = world();
    for modifier in crate::overslime::code_modifiers().unwrap() { world.registry.register_static(modifier).unwrap(); }
    helmet_env!(world, env);
    let (tags, _) = DamageTags::load(&damage_tag_stacks());
    let (_, mut tool) = armor_helmet(&world, &[("overslime", 1), ("overshield", 1)]);
    crate::overslime::set_amount(&mut tool.custom_data, 10);
    assert_eq!(crate::overslime::amount(&tool.custom_data), 10, "the overslime modifier gives the piece room for it");
    let run = |damage_type: &str, tool: &pluto_lib::ItemSlot| {
        let damage_type = Identifier::parse(damage_type).unwrap();
        let source = DamageSource { damage_type: &damage_type, indirect: false, tags: &tags, attacker_in_tag: &|_| false, entities: Default::default() };
        protection_with_edits(&tool.custom_data, &env, &source, 0.0)
    };
    // 1.25 a level, for the 2 overslime a level it spends, out of the 10 on the piece.
    let (value, edits) = run("minecraft:mob_attack", &tool);
    let (base, _) = { let (plain, _) = armor_helmet(&world, &[("overslime", 1)]); run("minecraft:mob_attack", &plain) };
    assert!((value - base - 1.25).abs() < 1e-4, "{value} {base}");
    assert_eq!(edits, vec![StackDataEdit { path: vec![TAG_PERSISTENT.into(), "tconstruct:overslime".into()], value: Some(Nbt::Int(8)) }]);
    // With only 1 left it gives half of the protection and uses it up (the key goes).
    crate::overslime::set_amount(&mut tool.custom_data, 1);
    let (value, edits) = run("minecraft:mob_attack", &tool);
    assert!((value - base - 0.625).abs() < 1e-4, "{value} {base}");
    assert_eq!(edits, vec![StackDataEdit { path: vec![TAG_PERSISTENT.into(), "tconstruct:overslime".into()], value: None }]);
    // Damage that bypasses invulnerability is not protected against and spends nothing.
    let (value, edits) = run("minecraft:out_of_world", &tool);
    assert!((value - base).abs() < 1e-4 && edits.is_empty());
}

#[test]
fn shipped_rebuff_heals_by_regeneration_and_wears_the_piece_while_restore_stays_before_armor() {
    use crate::{damage_tags::DamageTags, protection::DamageSource, tool_stack::restores, wearer_modules::{wear_edits, Restore}};
    use pluto_lib::{Nbt, StackDataEdit};
    let world = world();
    helmet_env!(world, env);
    let (tags, _) = DamageTags::load(&damage_tag_stacks());
    let damage_type = Identifier::parse("minecraft:mob_attack").unwrap();
    let source = DamageSource { damage_type: &damage_type, indirect: false, tags: &tags, attacker_in_tag: &|_| false, entities: Default::default() };
    let (tool, _) = armor_helmet(&world, &[("rebuff", 2)]);
    // Rebuff always lands; level 2 heals half the damage as regeneration at effect level 2 (25 ticks a point) and costs 2 wear.
    let found = restores(&tool.custom_data, &env, &source, 10.0, false, &mut || 0.0);
    assert_eq!(found, vec![(Identifier::parse("tconstruct:rebuff").unwrap(), Restore { duration: 125, wear: 2 })]);
    assert!(restores(&tool.custom_data, &env, &source, 1.0, false, &mut || 0.0).is_empty(), "half of 1 damage is not a whole point of health");
    assert!(restores(&tool.custom_data, &env, &source, 10.0, true, &mut || 0.0).is_empty(), "rebuff is not bound to the hurt hook");
    // The wear edits put the piece's damage up by what the module costs.
    let edits = wear_edits(&tool.custom_data, &env, &Identifier::parse("tconstruct:rebuff").unwrap(), 2, &mut || 1.0);
    assert!(edits.contains(&StackDataEdit { path: vec!["Damage".into()], value: Some(Nbt::Int(2)) }), "{edits:?}");
    // Restore is the same module on the other hook: nothing at the damage stage, and at the hurt stage a share of the
    // damage as it arrives (25% of 10 is 2 whole points of health, 50 ticks each at level I, one wear per level).
    let (restore, _) = armor_helmet(&world, &[("restore", 1)]);
    assert!(restores(&restore.custom_data, &env, &source, 10.0, false, &mut || 0.0).is_empty());
    assert_eq!(restores(&restore.custom_data, &env, &source, 10.0, true, &mut || 0.0), vec![(Identifier::parse("tconstruct:restore").unwrap(), Restore { duration: 100, wear: 1 })]);
}

#[test]
fn shipped_entangled_hurts_and_boon_of_sssss_shortens_the_wearers_beneficial_effects_when_the_piece_comes_off() {
    use crate::tool_stack::unequip_effects;
    use pluto_lib::EffectCategory;
    let world = world();
    helmet_env!(world, env);
    let on_unequip = |modifiers: &[(&str, i32)]| unequip_effects(&armor_helmet(&world, modifiers).0.custom_data, &env);
    assert_eq!(on_unequip(&[("entangled", 1)]).0, [("tconstruct:entangled".to_owned(), 1.0)]);
    assert_eq!(on_unequip(&[("entwined", 1)]).0, [("tconstruct:entangled".to_owned(), 2.0)], "entwined uses the entangled damage type for 2");
    let (hurts, cuts) = on_unequip(&[("boon_of_sssss", 2)]);
    assert!(hurts.is_empty());
    assert_eq!(cuts.len(), 1);
    assert_eq!(cuts[0].0, EffectCategory::Beneficial);
    assert!((cuts[0].1 - 0.4).abs() < 1e-5, "0.2 of the time a level");
    assert!(on_unequip(&[("sharpness", 1)]) == (Vec::new(), Vec::new()));
}

#[test]
fn shipped_solar_powered_cancels_tool_wear_by_the_sky_light_around_the_holder() {
    use crate::tool_stack::reduce_damage;
    use pluto_lib::EntityState;
    let world = world();
    helmet_env!(world, env);
    let (tool, _) = armor_helmet(&world, &[("solar_powered", 1)]);
    // (0.01 * level + 0.04) of the sky light, as a chance for each point: 5 times the light as a percentage.
    let dealt = |holder: Option<&EntityState>, roll: f32| crate::entity_facts::with_holder(holder, || reduce_damage(&tool.custom_data, &env, None, 5, &mut || roll));
    let sky = |light: u8, darken: u8| EntityState { sky_light: Some(light), sky_darken: darken, ..EntityState::default() };
    assert_eq!(dealt(Some(&sky(15, 11)), 0.1), 0, "light 4 is a 20% chance and a roll of 0.1 is under it");
    assert_eq!(dealt(Some(&sky(15, 11)), 0.5), 5, "a roll of 0.5 is not");
    assert_eq!(dealt(Some(&sky(15, 0)), 0.5), 0, "full daylight is a 75% chance");
    assert_eq!(dealt(Some(&sky(0, 0)), 0.0), 5, "no sky light, no reduction");
    assert_eq!(dealt(None, 0.5), 0, "nobody named: the fallback light of 15");
}

pub(crate) fn fluid_tag_files() -> Vec<(Identifier, Value)> {
    let base = root().join("fluid_tags");
    let mut found = Vec::new();
    for namespace in std::fs::read_dir(&base).expect("fluid tag folder") {
        let namespace = namespace.unwrap().path();
        let space = namespace.file_name().unwrap().to_str().unwrap().to_owned();
        fn walk(dir: &std::path::Path, relative: String, space: &str, out: &mut Vec<(Identifier, Value)>) {
            for entry in std::fs::read_dir(dir).expect("folder") {
                let path = entry.unwrap().path();
                let name = path.file_name().unwrap().to_str().unwrap().to_owned();
                if path.is_dir() { walk(&path, format!("{relative}{name}/"), space, out); continue; }
                if let Some(stem) = name.strip_suffix(".json") {
                    out.push((Identifier::new(space, &format!("{relative}{stem}")).unwrap(), serde_json::from_slice(&std::fs::read(&path).unwrap()).expect("valid JSON")));
                }
            }
        }
        walk(&namespace, String::new(), &space, &mut found);
    }
    found
}

/// Every condition decodes through the production tree and holds: what a pack that has every other mod's ingots would load.
pub(crate) struct EverythingOn(crate::conditions::Context);
impl crate::materials::Conditions for EverythingOn {
    type Parsed = crate::conditions::Condition;
    fn decode(&self, json: &Value) -> Result<Self::Parsed, String> { self.0.decode(json) }
    fn test(&self, _: &Self::Parsed) -> Result<bool, String> { Ok(true) }
}

pub(crate) fn fluid_tags() -> crate::fluid_ingredient::FluidTags { crate::fluid_ingredient::FluidTags::load(&fluid_tag_files()) }
pub(crate) fn fluid_recipes(everything_on: bool) -> crate::fluid_recipes::Recipes {
    let files = recursive_files("fluids", "fluids");
    assert_eq!(files.len(), 1858, "melting, casting, alloy, fuel, entity and molding recipes outside the compat folder");
    if everything_on { crate::fluid_recipes::Recipes::load(&files, &EverythingOn(conditions())) } else { crate::fluid_recipes::Recipes::load(&files, &conditions()) }
}
pub(crate) fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

#[test]
fn shipped_fluid_tags_flatten_and_only_two_unshipped_references_are_left_out() {
    let files = fluid_tag_files();
    assert_eq!(files.len(), 131, "the tconstruct, forge and mantle fluid tag files, and Mantle's own water tag");
    let tags = fluid_tags();
    assert_eq!(tags.problems, vec![
        "fluid tag tconstruct:chemthrower_effects/entity: unknown tag forge:milk".to_owned(),
        "fluid tag tconstruct:hide_in_creative_tanks: unknown tag c:hidden_from_recipe_viewers".to_owned()], "milk is a Forge tag and the recipe viewer tag a Fabric one");
    // the Forge tag names the Tinkers tag, which lists the source fluid before the flowing one
    assert_eq!(tags.members(&id("forge:molten_iron")).unwrap(), &vec![id("tconstruct:molten_iron"), id("tconstruct:flowing_molten_iron")]);
    assert!(tags.contains(&id("tconstruct:flowing_molten_debris"), &id("tconstruct:molten_debris")));
}

#[test]
fn every_shipped_fluid_recipe_decodes_whatever_its_conditions_and_only_unported_ingredients_are_left_out() {
    let recipes = fluid_recipes(true);
    let mut problems = recipes.problems.clone();
    problems.sort();
    // A potion display cast matches the item whatever potion it carries, and a block tag cast reads the item tag the importer writes for the block tag.
    assert!(problems.is_empty(), "{problems:#?}");
    // the tool domain's casting recipes decode too: 21 tool casting, 10 part swapping, 2 tipping and 2 tipped clearing
    let (mut tool, mut swapping, mut tipping, mut clearing) = (0, 0, 0, 0);
    for recipe in &recipes.casting {
        match &recipe.kind {
            crate::casting_recipes::CastingKind::Tool { .. } => tool += 1,
            crate::casting_recipes::CastingKind::PartSwap { .. } => swapping += 1,
            crate::casting_recipes::CastingKind::Tipping { .. } => tipping += 1,
            crate::casting_recipes::CastingKind::TippedClearing { .. } => clearing += 1,
            _ => {}
        }
    }
    assert_eq!((tool, swapping, tipping, clearing), (21, 10, 2, 2));
    // 481 melting + 155 ore + 183 damageable; 24 alloys (netherite's two forms are one recipe); all 689 casting recipes decode
    assert_eq!((recipes.melting.len(), recipes.material_melting.len(), recipes.fuels.len(), recipes.solid_fuel.is_some(), recipes.entity_melting.len(), recipes.alloys.len()), (819, 48, 2, true, 32, 24));
    assert_eq!((recipes.casting.len(), recipes.molding.len(), recipes.material_casting.casting.len(), recipes.material_casting.composite.len(), recipes.part_builder.len()), (689, 58, 39, 34, 112));
}

#[test]
fn shipped_melting_and_cooling_times_follow_the_original_formula() {
    use crate::{casting_recipes::CastingKind, fluid_ingredient::Source, fluid_values::{calc_cooling_time, calc_time_for_amount}, melting_recipes::MeltingKind};
    let recipes = fluid_recipes(true);
    // The generator gives a plain melting recipe sqrt(ingots) * T^0.585 unless its author overrode the time (clay, glass, ores...).
    let plain: Vec<_> = recipes.melting.iter().filter(|recipe| recipe.kind == MeltingKind::Plain).collect();
    let agree = plain.iter().filter(|recipe| calc_time_for_amount(recipe.temperature, recipe.output.amount) == recipe.time).count();
    assert_eq!((plain.len(), agree), (481, 293), "the other 188 carry an authored time");
    // Casting uses the same formula on the casting fluid's temperature, from the fluid's material_fluid recipe.
    let temperature_of = |source: &Source| recipes.material_casting.casting.iter()
        .find(|fluid| fluid.fluid.options.iter().any(|(own, _)| own == source)).map(|fluid| fluid.temperature);
    let (mut agree_cast, mut bottles, mut other) = (0, 0, Vec::new());
    for recipe in &recipes.casting {
        let (CastingKind::Item { .. }, Some(fluid)) = (&recipe.kind, &recipe.fluid) else { continue };
        let [(source, amount)] = fluid.options.as_slice() else { continue };
        let Some(temperature) = temperature_of(source) else { continue };
        if calc_cooling_time(temperature, *amount) == recipe.cooling_time { agree_cast += 1; }
        else if recipe.cooling_time == 1 { bottles += 1; }
        else { other.push(recipe.id.to_string()); }
    }
    assert_eq!((agree_cast, bottles, other.len()), (338, 7, 0), "the 7 bottle casts take 1 tick by design: {other:?}");
}

#[test]
fn a_shipped_iron_ingot_melts_in_a_melter_on_lava_and_the_fluid_alloys_in_a_smeltery() {
    use crate::{fluid_values::OreRate, heating::{Finished, Fuel, MeltingSlot}, melting_recipes::AlloyTank};
    let recipes = fluid_recipes(false);
    let tags = fluid_tags();
    let in_item_tag = |item: &Identifier, tag: &Identifier| tag.to_string() == "forge:ingots/iron" && item.path() == "iron_ingot";
    let in_fluid_tag = |fluid: &Identifier, tag: &Identifier| tags.contains(fluid, tag);
    let ingot = recipes.find_melting(&id("minecraft:iron_ingot"), &in_item_tag, false).expect("the shipped recipe melts an iron ingot");
    assert_eq!((ingot.temperature, ingot.time, ingot.output.amount), (800, 60, 90));
    assert_eq!(ingot.output.resolve(&tags), Some(id("tconstruct:molten_iron")), "the forge tag prefers the source fluid");
    assert!(recipes.find_melting(&id("minecraft:iron_nugget"), &in_item_tag, false).is_none(), "an item no recipe names does not melt");

    // The shipped lava fuel: 50 mB burns for 100 at 1000 C and rate 10. An ingot needs 600 progress, so 60 passes of
    // four ticks and one more to finish: 244 ticks, a little over 12 seconds.
    let lava = recipes.find_fuel(&id("minecraft:lava"), &in_fluid_tag).expect("lava is a fuel");
    assert_eq!((lava.amount(&id("minecraft:lava"), &in_fluid_tag), lava.duration, lava.temperature, lava.rate), (50, 100, 1000, 10));
    let mut fuel = Fuel::default();
    assert_eq!(fuel.try_liquid(50, 50, lava.duration, lava.temperature, lava.rate, true), 1000);
    let mut slot = MeltingSlot::default();
    slot.set_stack(Some((Some((ingot.time, ingot.temperature)), false)));
    let (mut passes, mut fluid) = (0, 0);
    while slot.occupied {
        slot.heat(fuel.temperature, fuel.rate, &mut || { fluid = ingot.output_amount(&OreRate::MELTER, None); Finished::Filled });
        fuel.decrease(1);
        passes += 1;
        assert!(passes < 200, "the slot never finished");
    }
    assert_eq!((passes, fluid), (61, 90));
    assert!(fuel.has_fuel(), "100 burn units outlast the 61 passes: one fill of lava smelts an ingot with fuel to spare");

    // Rose gold: copper and gold, 90 each, at 550 C, make 180. The smeltery holds both and a lot of iron.
    struct Tank { fluids: Vec<(Identifier, i32)>, temperature: i32 }
    impl AlloyTank for Tank {
        fn tanks(&self) -> usize { self.fluids.len() }
        fn fluid_in_tank(&self, tank: usize) -> Option<(Identifier, i32)> { self.fluids.get(tank).filter(|(_, amount)| *amount > 0).cloned() }
        fn temperature(&self) -> i32 { self.temperature }
        fn can_fit(&self, output: &(Identifier, i32), removed: i32) -> bool { output.1 - removed <= 10_000 - self.fluids.iter().map(|(_, amount)| amount).sum::<i32>() }
        fn drain(&mut self, _: usize, fluid: &Identifier, amount: i32) -> i32 {
            let entry = self.fluids.iter_mut().find(|(own, _)| own == fluid).unwrap();
            entry.1 -= amount;
            amount
        }
        fn fill(&mut self, fluid: &Identifier, amount: i32) -> i32 {
            match self.fluids.iter_mut().find(|(own, _)| own == fluid) { Some(entry) => entry.1 += amount, None => self.fluids.push((fluid.clone(), amount)) }
            amount
        }
    }
    let mut tank = Tank { fluids: vec![(id("tconstruct:molten_iron"), 900), (id("tconstruct:molten_gold"), 200), (id("tconstruct:molten_copper"), 90)], temperature: 1000 };
    let matching: Vec<String> = recipes.matching_alloys(&tank, &tags).iter().map(|alloy| alloy.id.to_string()).collect();
    assert_eq!(matching, vec!["tconstruct:fluids/smeltery/alloys/molten_rose_gold".to_owned()], "only copper plus gold is in the tank");
    let rose = recipes.matching_alloys(&tank, &tags)[0];
    assert!(rose.perform(&mut tank, &tags));
    assert_eq!(tank.fluids, vec![(id("tconstruct:molten_iron"), 900), (id("tconstruct:molten_gold"), 110), (id("tconstruct:molten_copper"), 0), (id("tconstruct:molten_rose_gold"), 180)]);
    assert!(!rose.can_perform(&tank, &tags), "the copper is used up");
    // an alloy is also refused below its temperature
    tank.fluids[2].1 = 90;
    assert!(rose.can_perform(&tank, &tags));
    tank.temperature = 549;
    assert!(!rose.can_perform(&tank, &tags));
}

#[test]
fn the_shipped_netherite_alloy_is_the_cheaper_one_by_default_and_obsidian_keeps_its_water() {
    let tags = fluid_tags();
    let recipes = fluid_recipes(false);
    let netherite = recipes.alloys.iter().find(|alloy| alloy.id.path().ends_with("molten_netherite")).expect("netherite alloy");
    let in_tag = |fluid: &Identifier, tag: &Identifier| tags.contains(fluid, tag);
    let amounts: Vec<i32> = netherite.inputs.iter().map(|input| input.fluid.options[0].1).collect();
    assert_eq!(amounts, vec![40, 20], "cheaper_netherite_alloy defaults on: 20 gold for 10 netherite instead of 40");
    assert_eq!(netherite.temperature, 1250);
    let obsidian = recipes.alloys.iter().find(|alloy| alloy.id.path().ends_with("molten_obsidian")).expect("obsidian alloy");
    assert_eq!(obsidian.inputs.iter().map(|input| input.catalyst).collect::<Vec<_>>(), vec![true, false]);
    assert!(obsidian.inputs[0].fluid.test(&id("minecraft:water"), 250, &in_tag) && !obsidian.inputs[0].fluid.test(&id("minecraft:water"), 249, &in_tag));
}

#[test]
fn a_shipped_pick_head_melts_by_its_casting_cost_and_a_golem_melts_into_iron() {
    use crate::tool_materials::MaterialVariant;
    let tags = fluid_tags();
    let recipes = fluid_recipes(false);
    let in_tag = |fluid: &Identifier, tag: &Identifier| tags.contains(fluid, tag);
    // A pick head costs two ingots of its material to cast (table_casting_material), so it melts into 180 mB in the time of a 180 mB casting.
    let head = id("tconstruct:pick_head");
    let iron = MaterialVariant::parse("tconstruct:iron").unwrap();
    let (melting, cost) = recipes.find_material_melting(&head, &iron).expect("iron parts melt");
    assert_eq!((cost, melting.temperature, melting.output_amount(cost), melting.time(cost)), (2, 800, 180, 85));
    assert!(recipes.find_material_melting(&id("minecraft:stick"), &iron).is_none(), "an item that is not a casting part does not melt by material");
    assert!(recipes.find_material_melting(&head, &MaterialVariant::parse("tconstruct:wood").unwrap()).is_none(), "wood has no molten form");
    // The casting side agrees: iron is the fluid that casts into iron, at the same cost and cooling time.
    let casting = recipes.material_casting.casting_fluid(&id("tconstruct:molten_iron"), &in_tag).expect("molten iron casts a material");
    assert_eq!(casting.output.material_id(), Some(id("tconstruct:iron")));
    assert_eq!(crate::casting_recipes::MaterialCasting::cooling_time(casting, cost, &id("tconstruct:molten_iron"), &in_tag), melting.time(cost));
    let in_entity_tag = |_: &Identifier, _: &Identifier| false;
    let golem = recipes.entity_melting.iter().find(|recipe| recipe.entity.matches(&id("minecraft:iron_golem"), &in_entity_tag)).expect("golems melt");
    assert_eq!((golem.damage, golem.output.amount, golem.output.resolve(&tags)), (4, 10, Some(id("tconstruct:molten_iron"))));
    assert!(recipes.entity_melting.iter().all(|recipe| recipe.damage >= 1));
}

#[test]
fn shipped_sand_cast_patterns_and_molding_recipes_load() {
    use crate::part_builder_recipes::Recipes;
    let files = recursive_files("fluids", "fluids");
    let patterns = Recipes::load(&files, &EverythingOn(conditions()));
    assert!(patterns.problems.is_empty(), "{:#?}", patterns.problems);
    assert_eq!(patterns.len(), 112, "the sand, red sand and clay cast patterns of the part builder");
    // Pressing a cast into sand: a molding recipe on the casting table turns a part into its sand cast.
    let recipes = fluid_recipes(false);
    let sand_cast = recipes.molding.iter().find(|recipe| recipe.id.path() == "fluids/smeltery/sand_cast_pickup").expect("sand cast pickup");
    assert!(sand_cast.pattern.is_none() && !sand_cast.pattern_consumed);
    assert_eq!(sand_cast.result.source, crate::fluid_ingredient::Source::Id(id("tconstruct:blank_sand_cast")));
}

/// The Tinkers fluid tags as the host reads them (`data/tconstruct/tags/fluid`), by tag id.
fn host_fluid_tag_files() -> Vec<(Identifier, Value)> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content/data/tconstruct/tags/fluid");
    fn walk(dir: &std::path::Path, relative: String, out: &mut Vec<(Identifier, Value)>) {
        for entry in std::fs::read_dir(dir).expect("folder") {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_str().unwrap().to_owned();
            if path.is_dir() { walk(&path, format!("{relative}{name}/"), out); continue; }
            if let Some(stem) = name.strip_suffix(".json") {
                out.push((Identifier::new("tconstruct", &format!("{relative}{stem}")).unwrap(), serde_json::from_slice(&std::fs::read(&path).unwrap()).expect("valid JSON")));
            }
        }
    }
    let mut found = Vec::new();
    walk(&base, String::new(), &mut found);
    found
}

#[test]
fn the_72_original_fluids_are_defined_with_the_original_attributes_and_names() {
    use pluto_lib::FluidRegistry;
    crate::fluids::register();
    let mine: Vec<_> = FluidRegistry::registered().into_iter().filter(|fluid| fluid.id.namespace() == "tconstruct").collect();
    assert_eq!(mine.len(), 72);
    // Values read off TinkerFluids by hand: the helper defaults (slime 1600/1600, hot 2000/10000/1000K, cool and powder water's),
    // the setters after them, the inverted fluids' negative density and the potion's own numbers.
    let attributes = |name: &str| { let a = FluidRegistry::get(&format!("tconstruct:{name}")).unwrap().attributes; (a.temperature, a.luminance, a.density, a.viscosity, a.color) };
    assert_eq!(attributes("venom"), (310, 0, 1600, 1600, 0xFFFFFFFF));
    assert_eq!(attributes("magma"), (600, 3, 1600, 1600, 0xFFFFFFFF));
    assert_eq!(attributes("ichor"), (1000, 0, -1600, 1600, 0xFFFFFFFF));
    assert_eq!(attributes("beetroot_soup"), (400, 0, 1000, 1000, 0xFF84160D));
    assert_eq!(attributes("powdered_snow"), (270, 0, 1000, 1000, 0xFFFFFFFF));
    assert_eq!(attributes("potion"), (315, 0, 1100, 1100, 0xFFF800F8));
    assert_eq!(attributes("seared_stone"), (900, 6, 2000, 10000, 0xFF4F4A47));
    assert_eq!(attributes("blazing_blood"), (1800, 15, 3500, 10000, 0xFFFFFFFF));
    assert_eq!(attributes("molten_iron"), (1100, 12, 2000, 10000, 0xFFFFFFFF));
    assert_eq!(attributes("molten_cinderslime"), (1350, 10, -2000, 10000, 0xFFFFFFFF));
    assert_eq!(attributes("molten_bendalloy"), (400, 9, 2000, 10000, 0xFFD6D2C1));
    assert_eq!(attributes("fiery_liquid"), (1800, 15, 2000, 10000, 0xFFFFFFFF));
    // Every fluid is named in the shipped languages under the key the original uses.
    for language in ["en_us", "ja_jp"] {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../tconstruct/content/assets/tconstruct/lang/{language}.json"));
        let lang: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let missing: Vec<_> = mine.iter().map(|fluid| fluid.translation_key()).filter(|key| lang.get(key).is_none()).collect();
        assert!(missing.is_empty(), "{language}: {missing:?}");
    }
}

#[test]
fn every_original_language_file_is_shipped_with_the_originals_keys() {
    // The original ships 20 language files and the importer copies each one whole (only line
    // endings are normalised), so every shipped file must hold exactly the original's keys;
    // the counts are the originals' key counts. The original's lzh file starts with a UTF-8
    // BOM; the importer drops it as it normalises line endings, since language files are
    // parsed as strict UTF-8 JSON by the content checks and the game.
    let expected: [(&str, usize); 20] = [
        ("de_de", 1900), ("en_us", 3623), ("es_ar", 2399), ("es_cl", 3261), ("es_es", 2399),
        ("es_mx", 3233), ("fr_fr", 2688), ("id_id", 348), ("it_it", 1926), ("ja_jp", 3411),
        ("ko_kr", 2190), ("lzh", 3219), ("pl_pl", 1360), ("pt_br", 2783), ("ru_ru", 3425),
        ("sv_se", 1932), ("tr_tr", 2244), ("uk_ua", 3526), ("zh_cn", 3561), ("zh_tw", 2244),
    ];
    for (language, keys) in expected {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../tconstruct/content/assets/tconstruct/lang/{language}.json"));
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{language}: {e}"));
        let lang: Value = serde_json::from_slice(&bytes).unwrap();
        let object = lang.as_object().unwrap();
        assert_eq!(object.len(), keys, "{language}: key count differs from the original's");
        assert!(object.values().all(|value| value.is_string()), "{language}: a language file maps keys to plain strings");
    }
}

#[test]
fn the_host_fluid_tags_hold_only_defined_fluids_and_match_the_recipe_copy_without_flowing_variants() {
    let files = host_fluid_tag_files();
    assert_eq!(files.len(), 84, "the original's tconstruct fluid tag files; forge and mantle ones stay with the recipes");
    let defined: Vec<Identifier> = crate::fluids::definitions().into_iter().map(|def| def.id).collect();
    let host = crate::fluid_ingredient::FluidTags::load(&files);
    // Forge's milk and the Fabric recipe-viewer tag are missing, and both are optional in the host copy.
    assert!(host.problems.is_empty(), "{:?}", host.problems);
    let recipe_copy = fluid_tags();
    for (tag, _) in &files {
        let members = host.members(tag).unwrap();
        assert!(members.iter().all(|member| defined.contains(member) || member.to_string() == "minecraft:lava"), "{tag}: {members:?}");
        // The forge and mantle references are inlined, so the flattened members are the original's (the recipe copy does not
        // flatten the two tags that name forge:milk and the Fabric viewer tag, which the host copy marks optional).
        let Some(all) = recipe_copy.members(tag) else { assert!(["tconstruct:chemthrower_effects/entity", "tconstruct:hide_in_creative_tanks"].contains(&tag.to_string().as_str()), "{tag}"); continue };
        let original: Vec<_> = all.iter().filter(|member| !member.path().starts_with("flowing_")).cloned().collect();
        assert_eq!(members, &original, "{tag} differs from the original beyond the flowing variants");
    }
    let milkless = host.members(&id("tconstruct:chemthrower_effects/entity")).unwrap();
    assert!(milkless.contains(&id("tconstruct:meat_soup")) && milkless.contains(&id("tconstruct:molten_steeleaf")) && !milkless.iter().any(|m| m.path().contains("milk")));
    assert_eq!(host.members(&id("tconstruct:molten_iron")).unwrap(), &vec![id("tconstruct:molten_iron")]);
    assert!(host.contains(&id("tconstruct:molten_iron"), &id("tconstruct:chemthrower_effects/entity")));
    assert!(host.contains(&id("minecraft:lava"), &id("tconstruct:swasher/drowned")));
}

#[test]
fn a_melter_on_the_shipped_data_burns_coal_from_below_and_melts_an_ingot_and_a_part() {
    use crate::{heating::MELTER_CAPACITY, melter::{Live, State, Tank}, smeltery_data::Data, tool_materials::MaterialVariant};
    use pluto_lib::{with_neighbours, FluidStack, ItemSlot, MemoryNeighbourOps};
    let data = Data { recipes: fluid_recipes(false), tags: fluid_tags(), recipes_loaded: true, tags_loaded: true };
    assert!(data.loaded());
    let item_tags = pluto_lib::fixtures::registry_with_tags(&["minecraft:iron_ingot"], &[("forge:ingots/iron", &["minecraft:iron_ingot"])]).unwrap();
    // The heater below holds coal; the melter holds an iron ingot and an iron pick head.
    let mut world = MemoryNeighbourOps::default();
    world.containers.insert([0, 63, 0], vec![Some(ItemSlot::new(id("minecraft:coal"), 1))]);
    let mut head = ItemSlot::new(id("tconstruct:pick_head"), 1);
    crate::tool_materials::set_part_material(&mut head, &MaterialVariant::parse("tconstruct:iron").unwrap());
    let mut items = vec![Some(ItemSlot::new(id("minecraft:iron_ingot"), 1)), Some(head), None];
    let (mut state, mut tank) = (State::default(), Tank { fluid: None, capacity: MELTER_CAPACITY });
    with_neighbours(&mut world, || {
        let mut live = Live::new(&data, Some(&item_tags), [0, 64, 0]);
        // The shipped solid fuel gives 800 C at rate 8, and coal burns 1600 ticks of which a quarter counts.
        for _ in 0..3 { state.step(&mut items, &mut tank, &mut live); }
        assert_eq!((state.fuel.temperature, state.fuel.rate), (800, 8));
        assert_eq!((state.slots[0].required_time, state.slots[0].required_temp), (600, 800), "the shipped iron ingot recipe: time 60, 800 C");
        assert_eq!((state.slots[1].required_time, state.slots[1].required_temp), (850, 800), "a pick head melts in the time of 180 mB of iron");
        // 400 burn units last 400 passes: the ingot finishes at pass 76, the head (850 at rate 8) at pass 108
        for _ in 0..4 * 110 { state.step(&mut items, &mut tank, &mut live); }
    });
    assert!(items[0].is_none() && items[1].is_none(), "both melted");
    assert_eq!(tank.fluid, Some(FluidStack { fluid: id("tconstruct:molten_iron"), amount: 90 + 180, tag: None }));
    assert!(world.containers[&[0, 63, 0]][0].is_none(), "the one coal was burned");
}

/// The pouring loop on the shipped data: a tank of molten iron behind a faucet, a casting table below it with a cast on it.
struct Pour { tank: Option<pluto_lib::FluidStack>, state: crate::casting::State, items: Vec<Option<pluto_lib::ItemSlot>>, now: crate::casting::Live<'static> }
impl pluto_lib::NeighbourOps for Pour {
    fn item_slots(&mut self, _: pluto_lib::BlockPos, _: pluto_lib::Direction) -> Option<Vec<pluto_lib::NeighbourSlot>> { None }
    fn extract_item(&mut self, _: pluto_lib::BlockPos, _: pluto_lib::Direction, _: usize, _: u32) -> Option<pluto_lib::ItemSlot> { None }
    fn insert_item(&mut self, _: pluto_lib::BlockPos, _: pluto_lib::Direction, stack: pluto_lib::ItemSlot) -> Option<pluto_lib::ItemSlot> { Some(stack) }
    fn fluid_tanks(&mut self, pos: pluto_lib::BlockPos, _: pluto_lib::Direction) -> Option<Vec<pluto_lib::NeighbourTank>> {
        match pos {
            [0, 64, 1] => Some(vec![pluto_lib::NeighbourTank { fluid: self.tank.clone(), capacity: 4000 }]),
            [0, 63, 0] => Some(vec![pluto_lib::NeighbourTank { fluid: self.state.fluid.clone(), capacity: crate::casting::HOST_CAPACITY }]),
            _ => None,
        }
    }
    fn drain_fluid(&mut self, pos: pluto_lib::BlockPos, _: pluto_lib::Direction, amount: u32) -> Option<pluto_lib::FluidStack> {
        if pos != [0, 64, 1] { return None; }
        let held = self.tank.as_mut()?;
        let taken = amount.min(held.amount);
        held.amount -= taken;
        let out = pluto_lib::FluidStack { fluid: held.fluid.clone(), amount: taken, tag: held.tag.clone() };
        if held.amount == 0 { self.tank = None; }
        Some(out)
    }
    fn fill_fluid(&mut self, pos: pluto_lib::BlockPos, _: pluto_lib::Direction, stack: &pluto_lib::FluidStack, execute: bool) -> u32 {
        if pos != [0, 63, 0] { return 0; }
        // the table's fluid filler, over a copy when only asking (what BlockEntityTypeRegistry::fill_fluid does)
        let (mut state, mut items) = (self.state.clone(), self.items.clone());
        let taken = state.fill(&self.now, &mut items, stack, false);
        if execute { self.state = state; self.items = items; }
        taken
    }
    fn block_state(&mut self, pos: pluto_lib::BlockPos) -> Option<String> { (pos == [0, 64, 0]).then(|| "tconstruct:seared_faucet[facing=north]".to_owned()) }
}

#[test]
fn a_faucet_pours_shipped_molten_iron_onto_a_cast_and_the_table_makes_an_ingot_and_the_basin_a_block() {
    use crate::{casting::{State, Live, INPUT, OUTPUT}, casting_recipes::Station, faucet::{self, Around}, smeltery_data::Data};
    use pluto_lib::{with_neighbours, FluidStack, ItemSlot};
    let data: &'static Data = Box::leak(Box::new(Data { recipes: fluid_recipes(false), tags: fluid_tags(), recipes_loaded: true, tags_loaded: true }));
    let tags: &'static pluto_lib::RegistryLookup = Box::leak(Box::new(pluto_lib::fixtures::registry_with_tags(
        &["tconstruct:ingot_cast", "minecraft:iron_ingot", "minecraft:iron_block", "tconstruct:blank_sand_cast", "tconstruct:sand_cast"],
        &[("tconstruct:casts/multi_use/ingot", &["tconstruct:ingot_cast"]), ("forge:ingots/iron", &["minecraft:iron_ingot"]), ("forge:storage_blocks/iron", &["minecraft:iron_block"]),
          ("tconstruct:casts/sand", &["tconstruct:sand_cast"])]).unwrap()));
    let iron = id("tconstruct:molten_iron");
    let mut world = Pour {
        tank: Some(FluidStack { fluid: iron.clone(), amount: 300, tag: None }), state: State::default(), items: vec![Some(ItemSlot::new(id("tconstruct:ingot_cast"), 1)), None],
        now: Live { data, item_tags: Some(tags), station: Station::Table, materials: None, tool_defs: None, modifiers: None },
    };
    let mut faucet = faucet::State::default();
    with_neighbours(&mut world, || {
        let mut around = Around::of([0, 64, 0], Around::facing_of([0, 64, 0]));
        faucet.activate(&mut around);
    });
    assert_eq!(world.state.recipe.as_ref().map(|recipe| recipe.path()), Some("fluids/smeltery/casting/metal/iron/ingot_gold_cast"));
    // the recipe takes exactly 90, so the faucet drew exactly that much
    for _ in 0..20 {
        with_neighbours(&mut world, || faucet.tick(&mut Around::of([0, 64, 0], Around::facing_of([0, 64, 0]))));
    }
    assert_eq!((world.tank.clone().map(|tank| tank.amount), world.state.fluid.clone().map(|fluid| fluid.amount), world.state.cooling), (Some(210), Some(90), 60));
    for _ in 0..60 { world.state.tick(&world.now, &mut world.items); }
    assert_eq!(world.items[OUTPUT].as_ref().map(|stack| stack.item.to_string()), Some("minecraft:iron_ingot".to_owned()));
    assert_eq!(world.items[INPUT].as_ref().map(|stack| stack.item.to_string()), Some("tconstruct:ingot_cast".to_owned()));
    // the basin casts a block with no cast, and a sand cast on the table is picked up as a blank one
    let basin = Live { data, item_tags: Some(tags), station: Station::Basin, materials: None, tool_defs: None, modifiers: None };
    let (mut block, mut items) = (State::default(), vec![None, None]);
    assert_eq!(block.fill(&basin, &mut items, &FluidStack { fluid: iron, amount: 900, tag: None }, false), 810);
    for _ in 0..180 { block.tick(&basin, &mut items); }
    assert_eq!(items[OUTPUT].as_ref().map(|stack| stack.item.to_string()), Some("minecraft:iron_block".to_owned()));
    let (mut table, mut slots) = (State::default(), vec![Some(ItemSlot::new(id("tconstruct:sand_cast"), 1)), None]);
    let mut hand = None;
    table.interact(&world.now, &mut slots, &mut hand, &mut Vec::new());
    assert_eq!(hand.map(|stack| stack.item.to_string()), Some("tconstruct:blank_sand_cast".to_owned()), "the shipped sand cast pickup recipe");
}

/// The crafting, smelting, blasting and stonecutting recipes the importer converted from the original's smeltery, tables, common,
/// world, gadgets and compat recipe folders, in the game's recipe format: every one makes something this port declares (or vanilla), reads only items it
/// declares or vanilla ones, and a shaped one defines every key its pattern uses. The obtaining chain of the casting loop (grout,
/// seared bricks, seared stone, drains, ducts, tanks, table, basin, faucet, channel, melter, heater) is present, the scorched set
/// with it (the obsidian pane is declared now), and so are the metals' compaction and the clear glass dyeing of the commons,
/// the platforms, the cheese and the firewood families' fences, slabs and stairs (the gold platform is declared; the two
/// firewood blocks themselves are absent because the original makes them only by casting -- molten obsidian over a planks
/// cast for nahuatl, the blazewood basin casting -- and those casting recipes ship with the fluid recipes). The bamboo and
/// bone armor stands and the two reversed gold frame conversions of the gadgets are present too, and so is the station pack.
/// What is not here names the books or the slime crystals, none of which this port declares yet.
#[test]
fn the_smeltery_blocks_can_be_crafted_from_declared_things_only() {
    let content = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content/data/tconstruct");
    let declared = |name: &str| ["block", "item"].iter().any(|kind| content.join(format!("horizon/{kind}/{name}.json")).exists());
    let known = |id: &str| match id.split_once(':') { Some(("minecraft", _)) => true, Some(("tconstruct", name)) => declared(name), _ => false };
    fn collect(node: &Value, out: &mut Vec<String>) {
        match node {
            Value::String(text) => out.push(text.clone()),
            Value::Array(list) => list.iter().for_each(|entry| collect(entry, out)),
            _ => panic!("an ingredient is a string or a list: {node}"),
        }
    }
    let mut count = 0;
    let mut stack = vec![content.join("recipe")];
    let mut made = std::collections::BTreeSet::new();
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).expect("recipe folder") {
            let path = entry.unwrap().path();
            if path.is_dir() { stack.push(path); continue; }
            let recipe: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).expect("valid JSON");
            let result = recipe["result"]["id"].as_str().expect("a result id");
            assert!(known(result), "{path:?} makes {result}");
            made.insert(result.to_owned());
            let mut ingredients = Vec::new();
            match recipe["type"].as_str().unwrap() {
                "minecraft:crafting_shaped" => {
                    let key = recipe["key"].as_object().unwrap();
                    for row in recipe["pattern"].as_array().unwrap() {
                        for letter in row.as_str().unwrap().chars().filter(|letter| *letter != ' ') { assert!(key.contains_key(&letter.to_string()), "{path:?} uses {letter}"); }
                    }
                    key.values().for_each(|value| collect(value, &mut ingredients));
                }
                "minecraft:crafting_shapeless" => recipe["ingredients"].as_array().unwrap().iter().for_each(|value| collect(value, &mut ingredients)),
                "minecraft:smelting" | "minecraft:blasting" | "minecraft:stonecutting" => collect(&recipe["ingredient"], &mut ingredients),
                other => panic!("{path:?} has the unreviewed type {other}"),
            }
            for ingredient in ingredients.iter().filter(|text| !text.starts_with('#')) { assert!(known(ingredient), "{path:?} needs {ingredient}"); }
            count += 1;
        }
    }
    assert_eq!(count, 279, "the plain recipes the importer converts, as measured on the merged tree: 103 of the smeltery folder, 15 of the tables, 154 of the commons and 7 of the gadgets (punji and the ichor and magma cakes, with the gadgets lane, and the bamboo and bone armor stands and the two reversed gold frame conversions, with the fancy gadgets lane) (the copper can came with the fluid items, the four table recipes with the parts data, the seared chute with the chutes, the seared gauge and fluid cannon with the cannons and gauges, the lanterns and soul glass with the lanterns, the alloyer with the smeltery periphery, the casting tank with unit 6d, and the scorched set, the book substitute and the whole commons with the recipe unit, which declared the metals, the clear glass and the obsidian pane; the recipe remainder unit added the platforms, the cheese and the firewood, and with the gold platform the scorched basin; 10 more with the station pack, whose crafting station, part chest, tinkers chest, cast chest and two anvils the list below names; and the four book recipes of the commons, materials and you, puny smelting, fantastic foundry and tinkers gadgetry, with the books unit); the rest name items this port does not declare yet");
    for item in ["grout", "seared_brick", "seared_stone", "seared_bricks", "seared_drain", "seared_duct", "seared_fuel_tank", "seared_ingot_tank", "seared_fuel_gauge", "seared_table", "seared_basin",
                 "seared_faucet", "seared_channel", "seared_melter", "seared_casting_tank", "seared_heater", "scorched_brick", "nether_grout", "scorched_duct", "scorched_fuel_tank", "scorched_channel", "scorched_alloyer", "seared_chute",
                 "seared_glass", "seared_ladder", "seared_stone_slab", "seared_bricks_wall", "scorched_bricks_fence", "blank_sand_cast", "copper_can", "copper_gauge", "seared_fluid_cannon",
                 "tinker_station", "part_builder", "modifier_worktable", "pattern", "crafting_station", "tinkers_chest", "part_chest", "cast_chest", "tinkers_anvil", "scorched_anvil",
                 "seared_lantern", "scorched_lantern", "seared_soul_glass", "seared_soul_glass_pane", "scorched_soul_glass", "scorched_soul_glass_pane",
                 "scorched_chute", "scorched_drain", "scorched_proxy_tank", "scorched_table", "end_fluid_cannon", "obsidian_gauge",
                 "cobalt_block", "cobalt_ingot", "copper_nugget", "clear_glass_pane", "gold_bars", "steel_ingot", "queens_slime_ingot", "sky_slime_ball",
                 "punji", "ichor_cake", "magma_cake", "bamboo_armor_stand", "bone_armor_stand", "gold_item_frame", "reversed_gold_item_frame", "gold_platform", "iron_platform", "cobalt_platform", "copper_platform", "waxed_copper_platform", "cheese_ingot", "cheese_block", "nahuatl_fence", "blazewood_stairs", "materials_and_you", "puny_smelting", "fantastic_foundry", "tinkers_gadgetry"] {
        assert!(made.contains(&format!("tconstruct:{item}")), "{item} is not craftable");
    }
}

#[test]
fn the_forge_item_tags_the_host_lacks_come_from_the_original_data() {
    // The original runs on Forge, whose own tags (forge:ingots/iron and family) its recipes name. The host ships
    // none of them, so the captured item snapshot gets them from the importer's table of the original data (common::capture_items).
    let items = crate::common::with_convention_tags(pluto_lib::fixtures::registry(&[
        "minecraft:iron_ingot", "minecraft:iron_nugget", "minecraft:iron_block", "minecraft:raw_iron",
        "minecraft:iron_ore", "minecraft:deepslate_iron_ore", "minecraft:gold_ingot", "minecraft:diamond", "minecraft:lapis_lazuli",
    ]).unwrap());
    let member = |tag: &str, item: &str| items.get_tag(&Identifier::parse(tag).unwrap()).is_some_and(|members| members.contains(&Identifier::parse(item).unwrap()));
    assert!(member("forge:ingots/iron", "minecraft:iron_ingot"));
    assert!(member("forge:nuggets/iron", "minecraft:iron_nugget"));
    assert!(member("forge:storage_blocks/iron", "minecraft:iron_block"));
    assert!(member("forge:raw_materials/iron", "minecraft:raw_iron"));
    assert!(member("forge:ores/iron", "minecraft:iron_ore"));
    assert!(member("forge:ores/iron", "minecraft:deepslate_iron_ore"), "the deepslate prefix is part of the ore, not the metal");
    assert!(member("forge:gems/diamond", "minecraft:diamond"));
    assert!(member("forge:gems/lapis", "minecraft:lapis_lazuli"), "Forge names the tag lapis, not lapis_lazuli");
    assert!(!member("forge:ingots/iron", "minecraft:gold_ingot"), "one metal's tag holds no other metal");
}

#[test]
fn shipped_fluid_recipes_load_and_an_iron_ingot_melts_through_its_forge_tag() {
    let files = recursive_files("fluids", "fluids");
    assert_eq!(files.len(), 1858, "every file the smeltery domain ships under recipes/fluids");
    let recipes = crate::fluid_recipes::Recipes::load(&files, &conditions());
    // 813 melting files ship, but most are switched off in this tag-less test world by their own conditions
    // (the tag-filled and other-mod gates of conditions()); what loads unconditionally must still be hundreds.
    assert!(recipes.melting_count() > 250, "{} melting recipes loaded, {} problems", recipes.melting_count(), recipes.problems.len());
    assert!(recipes.solid_fuel.is_some(), "the shipped solid fuel recipe (MeltingFuelLookup.getSolid) loaded");
    assert!(!recipes.fuels.is_empty(), "the shipped liquid fuel recipes loaded");
    // The shipped iron ingot recipe names the tag forge:ingots/iron, not the item: with no such tag the melter
    // finds no recipe for the ingot, never takes fuel and never heats (the real-game melter that sat at fuel 0).
    let ingot = Identifier::parse("minecraft:iron_ingot").unwrap();
    assert!(recipes.find_melting(&ingot, &|_, _| false, false).is_none(), "no tag, no melting recipe");
    let items = crate::common::with_convention_tags(pluto_lib::fixtures::registry(&["minecraft:iron_ingot"]).unwrap());
    let in_tag = |item: &Identifier, tag: &Identifier| items.get_tag(tag).is_some_and(|members| members.contains(item));
    let recipe = recipes.find_melting(&ingot, &in_tag, false).expect("the ingot melts once its forge tag exists");
    assert_eq!((recipe.temperature, recipe.time), (800, 60), "the shipped iron ingot melting values");
}

/// The world fluid content the importer wrote agrees with what PlutoLib flows: for each of the 70 fluids with a block, the block `<name>_fluid`
/// declares the `level` property with the values PlutoLib reads (and an outline on the source only), its blockstate maps every level to a model
/// that exists, and its bucket item is declared, modelled and defined; none is declared for the potion and powdered snow.
#[test]
fn the_world_fluid_content_matches_what_plutolib_flows() {
    use pluto_lib::{FluidRegistry, FluidWorld};
    crate::fluids::register();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content");
    let read = |relative: String| -> Value { serde_json::from_slice(&std::fs::read(root.join(&relative)).unwrap_or_else(|_| panic!("{relative} is missing"))).expect("valid JSON") };
    let mut checked = 0;
    for def in FluidRegistry::registered().into_iter().filter(|fluid| fluid.id.namespace() == "tconstruct") {
        let name = def.id.path().to_owned();
        let block = FluidWorld::block_id(&def.id);
        let declared = root.join(format!("data/tconstruct/horizon/block/{}.json", block.path())).exists();
        assert_eq!(declared, def.world_blocks, "{name}: a block is declared exactly for the fluids PlutoLib flows");
        assert_eq!(root.join(format!("data/tconstruct/horizon/item/{name}_bucket.json")).exists(), def.bucket, "{name}: bucket");
        if !declared { continue; }
        let declaration = read(format!("data/tconstruct/horizon/block/{}.json", block.path()));
        let values: Vec<&str> = declaration["state_properties"][FluidWorld::LEVEL]["values"].as_array().unwrap().iter().map(|value| value.as_str().unwrap()).collect();
        assert_eq!(values, FluidWorld::BLOCK_LEVELS, "{name}: the level property");
        assert_eq!(declaration["default_state"][FluidWorld::LEVEL], "0");
        assert_eq!(declaration["collision_shapes"]["default"].as_array().unwrap().len(), 0, "{name}: no collision");
        for level in FluidWorld::BLOCK_LEVELS {
            let shape = declaration["shapes"]["values"][level].as_array().unwrap();
            assert_eq!(!shape.is_empty(), level == "0", "{name}: only the source has an outline");
        }
        assert_eq!(declaration["properties"]["light"].as_u64().unwrap_or(0), u64::from(crate::fluid_defs::FLUIDS.iter().find(|spec| spec.name == name).unwrap().block_light));
        let state = read(format!("assets/tconstruct/blockstates/{}.json", block.path()));
        for level in FluidWorld::BLOCK_LEVELS {
            let model = state["variants"][format!("level={level}")]["model"].as_str().unwrap();
            let path = model.strip_prefix("tconstruct:").unwrap();
            let model = read(format!("assets/tconstruct/models/{path}.json"));
            let texture = model["textures"]["fluid"].as_str().unwrap().strip_prefix("tconstruct:").unwrap();
            assert!(root.join(format!("assets/tconstruct/textures/{texture}.png")).exists(), "{name}: still texture {texture}");
        }
        assert!(root.join(format!("assets/tconstruct/items/{name}_bucket.json")).exists() && root.join(format!("assets/tconstruct/models/item/{name}_bucket.json")).exists());
        checked += 1;
    }
    assert_eq!(checked, 70);
}

/// The shipped modifier tags as resource stacks, nested folders and all (tags/modifiers/...).
fn modifier_tag_stacks() -> Vec<JsonResourceStack> {
    fn walk(dir: &std::path::Path, base: &std::path::Path, out: &mut Vec<JsonResourceStack>) {
        for entry in std::fs::read_dir(dir).expect("modifier tag folder") {
            let path = entry.unwrap().path();
            if path.is_dir() { walk(&path, base, out); continue; }
            if path.extension().and_then(|e| e.to_str()) != Some("json") { continue; }
            let rel = path.strip_prefix(base).unwrap().to_str().unwrap().replace('\\', "/");
            let value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).expect("valid JSON");
            out.push(JsonResourceStack { id: Identifier::new("tconstruct", rel.trim_end_matches(".json")).unwrap(), layers: vec![Ok(value)] });
        }
    }
    let base = root().join("tags/modifiers");
    let mut stacks = Vec::new();
    walk(&base, &base, &mut stacks);
    stacks.sort_by(|a, b| a.id.cmp(&b.id));
    stacks
}

fn enchantment_tag_stacks() -> Vec<JsonResourceStack> {
    let mut stacks = Vec::new();
    for entry in std::fs::read_dir(root().join("../tags/enchantment/modifier_like")).expect("enchantment tag folder") {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_str().unwrap().to_owned();
        if let Some(stem) = name.strip_suffix(".json") {
            let value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).expect("valid JSON");
            stacks.push(JsonResourceStack { id: Identifier::new("tconstruct", stem).unwrap(), layers: vec![Ok(value)] });
        }
    }
    stacks.sort_by(|a, b| a.id.cmp(&b.id));
    stacks
}

#[test]
fn shipped_enchantment_map_loads_all_57_mappings_and_resolves_like_the_original() {
    use crate::enchantment_map::EnchantmentMap;
    let mut world = world();
    let value: Value = serde_json::from_slice(&std::fs::read(root().join("enchantments_to_modifiers.json")).unwrap()).expect("valid JSON");
    let stack = JsonResourceStack { id: Identifier::parse("tconstruct:enchantments_to_modifiers").unwrap(), layers: vec![Ok(value)] };
    let tag_stacks = enchantment_tag_stacks();
    assert_eq!(tag_stacks.len(), 21, "the shipped modifier_like tags");
    let map = EnchantmentMap::load(Some(&stack), &tag_stacks, &|id| world.registry.get(id).is_some());
    assert!(map.problems.is_empty(), "{:?}", map.problems);
    assert_eq!(map.counts(), (36, 21), "the shipped file's 57 mappings");
    // The production wiring loads the same map into the registry without new problems.
    world.registry.load_enchantment_map(Some(&stack), &tag_stacks);
    assert!(world.registry.problems.is_empty(), "{:?}", world.registry.problems);
    assert_eq!(world.registry.enchantment_map.counts(), (36, 21));
    let id = |text: &str| Identifier::parse(text).unwrap();
    for (enchantment, modifier) in [("minecraft:efficiency", "tconstruct:haste"), ("minecraft:fortune", "tconstruct:luck"),
        ("minecraft:looting", "tconstruct:luck"), ("minecraft:luck_of_the_sea", "tconstruct:luck"), ("minecraft:infinity", "tconstruct:crystalshot"),
        ("minecraft:silk_touch", "tconstruct:silky"), ("minecraft:unbreaking", "tconstruct:reinforced"), ("minecraft:bane_of_arthropods", "tconstruct:bane_of_sssss"),
        ("minecraft:riptide", "tconstruct:drill_attack"), ("minecraft:impaling", "tconstruct:antiaquatic"), ("minecraft:piercing", "tconstruct:impaling"),
        ("minecraft:sharpness", "tconstruct:sharpness"), ("minecraft:protection", "tconstruct:protection")] {
        assert_eq!(map.modifier_for(&id(enchantment)), Some(&id(modifier)), "{enchantment}");
        assert_eq!(world.registry.modifier_for_enchantment(&id(enchantment)), Some(&id(modifier)), "{enchantment} through the registry");
    }
    // The tags name only other mods' enchantments, each optional: membership still resolves from the
    // shipped files, so a stack carrying one (a datapack could define it) converts through its tag.
    assert_eq!(map.modifier_for(&id("cyclic:auto_smelt")), Some(&id("tconstruct:autosmelt")));
    assert_eq!(map.modifier_for(&id("ensorcellation:hunter")), Some(&id("tconstruct:luck")));
    assert!(map.has_enchantment(&id("tconstruct:luck")));
    assert!(!map.has_enchantment(&id("tconstruct:overslime")));
    let equivalents = map.equivalent_enchantments(&|modifier| *modifier == id("tconstruct:luck"), &map.mapped_enchantments());
    assert_eq!(equivalents, vec![id("minecraft:fortune"), id("minecraft:looting"), id("minecraft:luck_of_the_sea"), id("ensorcellation:hunter")]);
}

#[test]
fn shipped_enchantment_converting_recipes_load_and_convert_a_book() {
    use crate::enchantment_converting::Conversions;
    use std::collections::BTreeMap;
    let mut world = world();
    let value: Value = serde_json::from_slice(&std::fs::read(root().join("enchantments_to_modifiers.json")).unwrap()).expect("valid JSON");
    let stack = JsonResourceStack { id: Identifier::parse("tconstruct:enchantments_to_modifiers").unwrap(), layers: vec![Ok(value)] };
    world.registry.load_enchantment_map(Some(&stack), &enchantment_tag_stacks());
    world.registry.load_tags(&modifier_tag_stacks());
    let modifier_in_tag = |modifier: &Identifier, tag: &Identifier| world.registry.reverse_tags.get(modifier).is_some_and(|tags| tags.contains(tag));
    let files = recursive_files("tools/modifiers", "tools/modifiers");
    let conversions = Conversions::load(&files, &conditions());
    assert!(conversions.problems.is_empty(), "{:?}", conversions.problems);
    assert_eq!(conversions.recipes.len(), 10, "the two unenchant recipes and the eight slot-type ones (abilities, upgrades, defense, slotless, as book and tool)");
    let book_recipe = conversions.recipes.iter().find(|recipe| recipe.match_book && recipe.name == "modifiers").expect("the book recipe");
    let tool_recipe = conversions.recipes.iter().find(|recipe| !recipe.match_book && recipe.name == "modifiers").expect("the tool recipe");
    assert!(book_recipe.return_unenchanted && tool_recipe.return_unenchanted);
    let id = |text: &str| Identifier::parse(text).unwrap();
    let in_tag = |_: &Identifier, _: &Identifier| false;
    let inputs = [Some((id("tconstruct:ender_slime_crystal"), 1)), Some((id("minecraft:dragon_breath"), 5))];
    let book = id("minecraft:enchanted_book");
    assert!(book_recipe.matches(&book, false, &inputs, &[], &in_tag));
    assert!(!tool_recipe.matches(&id("tconstruct:pickaxe"), false, &inputs, &[], &in_tag), "a Tinkers tool is not enchantable");
    assert!(tool_recipe.matches(&id("minecraft:diamond_pickaxe"), true, &inputs, &[], &in_tag), "an enchantable tool matches");
    // A fortune III book offers a level 1 luck crystal (the book returns), and comes back at fortune II.
    let modifier_recipes = crate::modifier_recipes::Recipes::load(&files, &conditions());
    let recipe_modifier = |slot: Option<&str>, modifier: &Identifier| modifier_recipes.is_recipe_modifier(slot, modifier);
    let enchantments: BTreeMap<String, i32> = [("minecraft:fortune".to_owned(), 3)].into_iter().collect();
    assert_eq!(book_recipe.options(&enchantments, &world.registry.enchantment_map, &in_tag, &recipe_modifier), vec![(id("tconstruct:luck"), 1)]);
    let crystal = book_recipe.result(&(id("tconstruct:luck"), 1), &inputs, &[], &in_tag).expect("the crystal");
    assert_eq!(crystal.custom_data.get("modifier"), Some(&pluto_lib::Nbt::String("tconstruct:luck".into())));
    let returned = book_recipe.unenchanted_return(&book, &enchantments, &world.registry.enchantment_map, &id("tconstruct:luck")).expect("the book returns");
    assert_eq!(returned.enchantments.get("minecraft:fortune"), Some(&2));
    // The slot-type recipes offer only modifiers a modifier recipe adds for their slot type: the
    // abilities recipe offers crystalshot for an infinity book (its recipes spend ability slots);
    // the slotless recipe offers nothing for the fortune book: luck's later levels are slotless
    // recipes, but luck sits on the slotless extraction blacklist.
    let ability_book = conversions.recipes.iter().find(|recipe| recipe.match_book && recipe.name == "abilities").expect("the abilities book recipe");
    let infinity: BTreeMap<String, i32> = [("minecraft:infinity".to_owned(), 1)].into_iter().collect();
    assert_eq!(ability_book.options(&infinity, &world.registry.enchantment_map, &modifier_in_tag, &recipe_modifier), vec![(id("tconstruct:crystalshot"), 1)]);
    let slotless_book = conversions.recipes.iter().find(|recipe| recipe.match_book && recipe.name == "slotless").expect("the slotless book recipe");
    assert!(slotless_book.options(&enchantments, &world.registry.enchantment_map, &modifier_in_tag, &recipe_modifier).is_empty());
}

#[test]
fn shipped_player_added_enchantments_count_in_the_pieces_vanilla_protection_points() {
    use crate::{damage_tags::DamageTags, protection::{host_enchantment_points, DamageSource},
        tool_stack::{carried_enchantments, enchantments, rebuild_stats, set_added_enchantments, Environment, TAG_MATERIALS}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let helmet = &world.tools.iter().find(|(id, _)| id.path() == "plate_helmet").unwrap().1;
    let helmet_id = Identifier::parse("tconstruct:plate_helmet").unwrap();
    let armor_tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:plate_helmet"], &[("tconstruct:modifiable/armor", &["tconstruct:plate_helmet"])]).unwrap();
    let env = Environment { definition: Some(helmet), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&helmet_id), item_tags: Some(&armor_tags) };
    let (tags, problems) = DamageTags::load(&damage_tag_stacks());
    assert!(problems.is_empty(), "{problems:?}");
    let mut tool = ItemSlot::new(helmet_id.clone(), 1);
    tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
    assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
    // The player added protection IV; whatever the piece's modifiers give, the carried set counts 4 more.
    let given = enchantments(&tool.custom_data, &env);
    set_added_enchantments(&mut tool.custom_data, &[("minecraft:protection".to_owned(), 4)].into_iter().collect());
    let carried = carried_enchantments(&tool.custom_data, &env);
    assert_eq!(carried.get("minecraft:protection").copied().unwrap_or(0), given.get("minecraft:protection").copied().unwrap_or(0) + 4);
    let damage_type = Identifier::parse("minecraft:mob_attack").unwrap();
    let source = DamageSource { damage_type: &damage_type, indirect: false, tags: &tags, attacker_in_tag: &|_| false, entities: Default::default() };
    assert_eq!(host_enchantment_points(&[carried], &source), host_enchantment_points(&[given], &source) + 4.0,
        "the hook's vanilla points include the player-added enchantment, so it is not protected twice");
}

#[test]
fn shipped_loot_modifiers_load_in_the_global_order_and_apply() {
    let mut shipped = files("../loot_modifiers");
    shipped.extend(files("../loot_modifiers/lustrous").into_iter().map(|(id, value)| {
        (Identifier::new("tconstruct", &format!("lustrous/{}", id.path())).unwrap(), value)
    }));
    assert_eq!(shipped.len(), 21);
    let modifiers = crate::loot_modifiers::Modifiers::load(&shipped);
    assert!(modifiers.problems.is_empty(), "{:?}", modifiers.problems);
    assert_eq!(modifiers.len(), 21);
    // The imported global list names the same modifiers in the same order the engine runs them.
    let global: Value = serde_json::from_slice(&std::fs::read(root().join("../../forge/loot_modifiers/global_loot_modifiers.json")).unwrap()).expect("valid JSON");
    let listed: Vec<&str> = global["entries"].as_array().unwrap().iter().map(|entry| entry.as_str().unwrap()).collect();
    assert_eq!(listed, crate::loot_modifiers::ORDER.to_vec());

    let id = |text: &str| Identifier::parse(text).unwrap();
    // An iron ore break with a lustrous tool: of all 21, only the iron lustrous modifier fires.
    let context = crate::loot_modifiers::Context {
        table: "minecraft:blocks/iron_ore",
        has_block: true,
        has_entity: false,
        entity: None,
        block_in_tag: &|tag| *tag == id("forge:ores/iron"),
        entity_in_tag: &|_| false,
        item_in_tag: &|item, tag| *item == id("minecraft:iron_ore") && *tag == id("forge:ores/iron"),
        tag_filled: &|_| true,
        tag_preference: &|tag| (*tag == id("forge:nuggets/iron")).then(|| id("minecraft:iron_nugget")),
        tool_level: &|modifier| if *modifier == id("tconstruct:lustrous") { 1 } else { 0 },
        tool_looting: 0,
        chrysophilite: 0.0,
        config: &|_| None,
        explosion: None,
    };
    let mut loot = vec![pluto_lib::ItemSlot::new(id("minecraft:raw_iron"), 1)];
    modifiers.apply(&context, &mut loot, &mut || 0.0);
    assert_eq!(loot.len(), 2);
    assert_eq!(loot[1].item, id("minecraft:iron_nugget"));
    assert_eq!(loot[1].count, 2);

    // A wither skeleton's bones become necrotic bones while the config allows it.
    let skeleton = id("minecraft:wither_skeleton");
    let context = crate::loot_modifiers::Context {
        table: "minecraft:entities/wither_skeleton",
        has_block: false,
        has_entity: true,
        entity: Some(&skeleton),
        block_in_tag: &|_| false,
        entity_in_tag: &|_| false,
        item_in_tag: &|_, _| false,
        tag_filled: &|_| false,
        tag_preference: &|_| None,
        tool_level: &|_| 0,
        tool_looting: 0,
        chrysophilite: 0.0,
        config: &|prop| (prop == "wither_bone_drop").then_some(true),
        explosion: None,
    };
    let mut loot = vec![pluto_lib::ItemSlot::new(id("minecraft:bone"), 2)];
    modifiers.apply(&context, &mut loot, &mut || 0.0);
    assert_eq!(loot[0].item, id("tconstruct:necrotic_bone"));
    assert_eq!(loot[0].count, 2);
}
#[test]
fn every_shipped_fluid_effect_decodes_and_the_conditioned_ones_wait_for_their_world() {
    let files = files("fluid_effects");
    assert_eq!(files.len(), 85, "every fluid effect file the original ships");
    let effects = crate::fluid_effects::Effects::load(&files, &conditions());
    assert!(effects.problems.is_empty(), "fluid effects that do not decode: {:?}", effects.problems);
    // The vanilla-only test world: the 40 files gated on another mod being loaded, on an integration
    // material being forced, or on another mod's fluid tag being filled stay unloaded, as in the game.
    assert_eq!(effects.len(), 45);
    let loaded: Vec<String> = effects.ids().map(ToString::to_string).collect();
    for present in ["blazing_blood", "milk", "water", "potion", "ender_slime", "molten_diamond", "molten_iron"] {
        assert!(loaded.contains(&format!("tconstruct:{present}")), "{present} should be loaded");
    }
    for absent in ["concrete", "potion_create", "molten_tin", "glowstone", "redstone"] {
        assert!(!loaded.contains(&format!("tconstruct:{absent}")), "{absent} should wait for its world");
    }
    // And the loaded ones are found by their fluid; a fluid whose file waits for its world finds none.
    let tags = fluid_tags();
    let in_tag = |fluid: &Identifier, tag: &Identifier| tags.contains(fluid, tag);
    let iron = pluto_lib::FluidStack { fluid: id("tconstruct:molten_iron"), amount: 1000, tag: None };
    assert!(effects.find(&iron, &in_tag).is_some());
    let lead = pluto_lib::FluidStack { fluid: id("tconstruct:molten_lead"), amount: 1000, tag: None };
    assert!(effects.find(&lead, &in_tag).is_none(), "molten lead's file waits for its world too");
}

/// The six books build from the Mantle data the importer ships: every section of each book's index is present (the
/// transformer-generated sections with their index page), and the file-backed pages are all carried over — the counts are
/// the original data's, counted page by page (file pages in English, one index page per generated section and per group
/// anchor, one blank page per blank or padding entry).
#[test]
fn the_six_books_build_from_their_shipped_mantle_data() {
    let assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content/assets/tconstruct");
    let read = |path: &str| std::fs::read(assets.join(path)).ok();
    let expected = [("materials_and_you", 8, 32), ("puny_smelting", 12, 58), ("mighty_smelting", 10, 48),
                    ("tinkers_gadgetry", 11, 38), ("fantastic_foundry", 8, 39), ("encyclopedia", 13, 40)];
    let mut total = 0;
    for (name, sections, pages) in expected {
        let book = crate::books::build_book(&read, name, "en_us").unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(book.sections.len(), sections, "{name}'s sections");
        assert_eq!(book.page_count(), pages, "{name}'s pages");
        assert!(book.sections.iter().all(|section| !section.pages.is_empty()), "{name} has an empty section");
        total += pages;
    }
    assert_eq!(total, 255);
    let first = crate::books::build_book(&read, "materials_and_you", "en_us").unwrap();
    assert_eq!(first.sections[0].name, "Introduction");
    assert_eq!(first.sections[0].pages[0].title(), Some("Materials & You"), "the welcome page opens the book");
    assert_eq!(first.appearance.export_title.as_deref(), Some("Materials and You"));
}
