//! Repair-kit repair outside the tinker station, the original's
//! `CraftingTableRepairKitRecipe` and `RepairKitItem.overrideStackedOnOther`
//! (SlimeKnights, MIT; see LICENSE), over PlutoLib's special-recipe and
//! stacked-on-other surfaces.
//!
//! Both shapes share the original's arithmetic: the kit's material must be
//! able to repair the tool (`MaterialRepairToolHook.canRepairWith`), the
//! repair amount is the hook's amount scaled by the kit's repair amount
//! over `MaterialRecipe.INGOTS_PER_REPAIR`, and the tool's modifiers pass
//! the total through the REPAIR_FACTOR chain, where a non-positive result
//! repairs nothing. In a crafting grid the pair is the whole recipe: one
//! durability tool and one repair kit, nothing else. In a menu, a
//! secondary click of the carried kit onto a modifiable tool is always
//! consumed, and repairs (consuming one kit and playing the saw sound)
//! only when the tool is damaged and the amount is positive.
//!
//! Documented reduction: the grid surface reports the cropped grid, so
//! the recipe's `canCraftInDimensions` (grid area at least 2, true of
//! every real crafting grid) is not re-checked. The menu sound plays as
//! the original plays it: volume 1 and pitch 0.8 + 0.4 * random, the
//! random drawn by the transport from the game's own stream.

use std::sync::Arc;

use pluto_lib::item_api_v1::{
    StackedOnOtherDecision, StackedOnOtherRegistry, StackedOnOtherRequest,
    StackedSound,
};
use pluto_lib::recipe_api_v1::{
    GridFacts, SpecialRecipeRegistration, SpecialResult, StackView,
};
use pluto_lib::{Identifier, ItemSlot, NbtCompound};

use crate::materials::Definitions as Materials;
use crate::modifiers::Registry;
use crate::tool_definitions::{self, Data};
use crate::tool_materials::MaterialVariant;
use crate::{tool_slot, tool_stack};

/// The recipe id of the shipped `tables/crafting_table_repair` recipe.
const RECIPE_ID: &str = "tconstruct:tables/crafting_table_repair";
/// The one `IRepairKitItem` item; every kit may repair in a crafting grid
/// (the interface's `canRepairInCraftingTable` default).
const REPAIR_KIT: &str = "tconstruct:repair_kit";
/// `TinkerTags.Items.DURABILITY`.
const DURABILITY_TAG: &str = "tconstruct:modifiable/durability";
/// `TinkerTags.Items.MODIFIABLE`.
const MODIFIABLE_TAG: &str = "tconstruct:modifiable";
/// `Sounds.SAW`, the little_saw sound event.
const SAW_SOUND: &str = "tconstruct:little_saw";
/// `MaterialRecipe.INGOTS_PER_REPAIR`.
const INGOTS_PER_REPAIR: f32 = 3.0;

/// The server's loaded data; None while any registry has not loaded, so a
/// handler never answers from partial state.
struct Loaded {
    definitions: Materials,
    modifiers: Registry,
    items: Option<Arc<pluto_lib::RegistryLookup>>,
}

fn load() -> Option<Loaded> {
    let definitions = crate::common::register().read().ok()?.clone();
    if !definitions.server_loaded {
        return None;
    }
    let modifiers = crate::common::server_modifiers()?.read().ok()?.clone();
    Some(Loaded { definitions, modifiers, items: crate::common::item_snapshot() })
}

impl Loaded {
    fn in_tag(&self, item: &Identifier, tag: &Identifier) -> bool {
        self.items
            .as_ref()
            .and_then(|items| items.get_tag(tag))
            .is_some_and(|members| members.contains(item))
    }
    fn environment<'a>(
        &'a self,
        item: &'a Identifier,
        definition: &'a Data,
    ) -> tool_stack::Environment<'a> {
        tool_stack::Environment {
            definition: Some(definition),
            has_materials: definition.material_stat_types().is_some(),
            materials: &self.definitions,
            modifiers: &self.modifiers,
            item_tags_loaded: self.items.is_some(),
            item: Some(item),
            item_tags: self.items.as_deref(),
        }
    }
}

fn tool_definition(item: &Identifier) -> Option<Data> {
    tool_definitions::register().read().ok()?.data.get(item).cloned()
}

/// A repair kit stack's material: the material the stack itself carries,
/// or None when it carries none (the original's UNKNOWN material).
fn kit_material(data: &NbtCompound) -> Option<Identifier> {
    let variant = crate::tool_materials::part_material(data);
    if variant == MaterialVariant::unknown() {
        return None;
    }
    variant.material_id()
}

/// The tool repaired by one kit of `material`, or None when the original
/// would not repair: the material cannot repair this tool, or the scaled
/// amount does not survive the REPAIR_FACTOR chain above zero.
fn repaired_tool(
    loaded: &Loaded,
    definition: &Data,
    tool: &ItemSlot,
    material: &Identifier,
    kit_amount: f32,
) -> Option<ItemSlot> {
    let env = loaded.environment(&tool.item, definition);
    let list = tool_stack::materials(
        &tool.custom_data,
        definition.material_stat_types().is_some(),
    );
    let can_repair = definition.is_repair_material(&list, material)
        || tool_stack::modifier_is_repair_material(&tool.custom_data, &env, material);
    if !can_repair {
        return None;
    }
    let amount = definition
        .repair_amount(&list, material, &loaded.definitions)
        .max(tool_stack::modifier_repair_amount(&tool.custom_data, &env, material) as f32);
    let amount = amount * (kit_amount / INGOTS_PER_REPAIR);
    let amount = tool_stack::repair_factor(&tool.custom_data, &env, amount);
    if amount <= 0.0 {
        return None;
    }
    let mut result = tool.clone();
    result.count = 1;
    tool_slot::repair_slot(&mut result, &env, amount as i32).ok()?;
    Some(result)
}

/// `CraftingTableRepairKitRecipe.matches` + `assemble` over the cropped
/// grid facts: exactly one repair kit and one durability tool, a damaged
/// or broken tool, and a kit material that repairs it.
fn grid_repair(
    loaded: &Loaded,
    kit_amount: f32,
    definition: &dyn Fn(&Identifier) -> Option<Data>,
    in_tag: &dyn Fn(&Identifier, &Identifier) -> bool,
    grid: &GridFacts<'_>,
) -> Option<SpecialResult> {
    let kit_id = Identifier::parse(REPAIR_KIT).ok()?;
    let durability = Identifier::parse(DURABILITY_TAG).ok()?;
    let mut tool: Option<&StackView<'_>> = None;
    let mut kit: Option<&StackView<'_>> = None;
    for cell in grid.cells.iter().flatten() {
        let item = Identifier::parse(cell.id).ok()?;
        if item == kit_id {
            if kit.is_some() {
                return None;
            }
            kit = Some(cell);
        } else if in_tag(&item, &durability) {
            if tool.is_some() {
                return None;
            }
            tool = Some(cell);
        } else {
            return None;
        }
    }
    let (tool, kit) = (tool?, kit?);
    let material = kit.custom_data.and_then(|data| data.as_compound()).and_then(kit_material)?;
    let tool_item = Identifier::parse(tool.id).ok()?;
    let definition = definition(&tool_item)?;
    let tool_data = tool.custom_data.and_then(|data| data.as_compound())?;
    if !tool_stack::is_broken(tool_data) && tool_stack::damage(tool_data) == 0 {
        return None;
    }
    let mut slot = ItemSlot::new(tool_item, tool.count);
    slot.damage = tool.damage;
    slot.custom_data = tool_data.clone();
    let repaired = repaired_tool(loaded, &definition, &slot, &material, kit_amount)?;
    Some(SpecialResult {
        item: tool.id.to_owned(),
        count: tool.count,
        damage: repaired.damage,
        custom_data: Some(repaired.custom_data),
        remainders: None,
    })
}

/// `RepairKitItem.overrideStackedOnOther`: a secondary click the slot
/// allows modification on, onto a modifiable tool, is always consumed;
/// the repair itself needs a damaged tool, a known kit material and a
/// positive amount, and takes one kit.
fn stacked_repair(
    loaded: &Loaded,
    kit_amount: f32,
    definition: &dyn Fn(&Identifier) -> Option<Data>,
    in_tag: &dyn Fn(&Identifier, &Identifier) -> bool,
    request: &StackedOnOtherRequest<'_>,
) -> StackedOnOtherDecision {
    use StackedOnOtherDecision as Decision;
    if !request.secondary || !request.allow_modification {
        return Decision::Pass;
    }
    let Some(clicked) = request.clicked else { return Decision::Pass };
    let modifiable = Identifier::parse(MODIFIABLE_TAG).expect("valid tag");
    if !in_tag(&clicked.item, &modifiable) {
        return Decision::Pass;
    }
    // From here the original always consumes the click, repair or not.
    if !tool_stack::is_broken(&clicked.custom_data) && tool_stack::damage(&clicked.custom_data) == 0 {
        return Decision::Consume;
    }
    let Some(material) = kit_material(&request.carried.custom_data) else {
        return Decision::Consume;
    };
    let Some(definition) = definition(&clicked.item) else {
        return Decision::Consume;
    };
    let Some(repaired) = repaired_tool(loaded, &definition, clicked, &material, kit_amount)
    else {
        return Decision::Consume;
    };
    let carried = if request.carried.count <= 1 {
        None
    } else {
        let mut rest = request.carried.clone();
        rest.count -= 1;
        Some(rest)
    };
    Decision::Apply { clicked: Some(repaired), carried, give: None, sound: Some(StackedSound { id: SAW_SOUND, volume: 1.0, pitch: 0.8 + 0.4 * request.random }) }
}

fn special_callback(grid: &GridFacts<'_>) -> Option<SpecialResult> {
    let loaded = load()?;
    let kit_amount = crate::config::repair_kit_amount().ok()?;
    grid_repair(&loaded, kit_amount, &tool_definition, &|item, tag| loaded.in_tag(item, tag), grid)
}

fn stacked_callback(request: &StackedOnOtherRequest<'_>) -> StackedOnOtherDecision {
    let Some(loaded) = load() else { return StackedOnOtherDecision::Pass };
    let Ok(kit_amount) = crate::config::repair_kit_amount() else {
        return StackedOnOtherDecision::Pass;
    };
    stacked_repair(
        &loaded,
        kit_amount,
        &tool_definition,
        &|item, tag| loaded.in_tag(item, tag),
        request,
    )
}

/// Installs the special recipe and the repair kit's stacked-on-other
/// behaviour.
pub(crate) fn register() {
    let _ = pluto_lib::recipe_api_v1::register_special_recipe(SpecialRecipeRegistration {
        id: RECIPE_ID,
        callback: special_callback,
    });
    StackedOnOtherRegistry::register(
        Identifier::parse(REPAIR_KIT).expect("valid item id"),
        stacked_callback,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::{Material, Rarity};
    use pluto_lib::{JsonResourceStack, Nbt};
    use serde_json::json;

    struct Always;
    impl crate::materials::Conditions for Always {
        type Parsed = ();
        fn decode(&self, _: &serde_json::Value) -> Result<(), String> { Ok(()) }
        fn test(&self, _: &()) -> Result<bool, String> { Ok(true) }
    }
    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn variant(text: &str) -> MaterialVariant { MaterialVariant::parse(text).unwrap() }

    struct World {
        loaded: Loaded,
        pickaxe: Data,
    }
    fn world() -> World {
        // A pickaxe whose head material repairs it (the head stat is the
        // repair amount), built from wood with head durability 40.
        let pickaxe = Data::decode(&json!({"modules":[
            {"type":"tconstruct:part_stats","parts":["tconstruct:pick_head","tconstruct:tool_handle"]},
            {"type":"tconstruct:material_repair","material":"test:wood","stat_type":"tconstruct:head"},
            {"type":"tconstruct:material_repair","material":"test:stone","stat_type":"tconstruct:head"}
        ]})).unwrap();
        let mut definitions = Materials::default();
        definitions.stats = crate::material_stats::load(
            &["test:wood", "test:stone"].map(|name| JsonResourceStack {
                id: id(name),
                layers: vec![Ok(json!({"stats":{
                    "tconstruct:head":{"durability":40,"mining_speed":3},
                    "tconstruct:handle":{"durability":1.0}}}))],
            }),
            &mut definitions.problems,
        );
        for name in ["test:wood", "test:stone"] {
            definitions.materials.insert(
                id(name),
                Material { id: id(name), tier: 1, sort_order: 1, rarity: Rarity::Common, craftable: true, hidden: false },
            );
        }
        definitions.server_loaded = true;
        let mut registry = Registry::default();
        registry.reload(&[], &Always);
        World {
            loaded: Loaded { definitions, modifiers: registry, items: None },
            pickaxe,
        }
    }

    fn tool(world: &World, damage: i32) -> ItemSlot {
        let item = id("tconstruct:pickaxe");
        // The station test context's shape: tags count as loaded with no
        // snapshot, so the build's stat rebuild is ready.
        let env = tool_stack::Environment {
            definition: Some(&world.pickaxe),
            has_materials: world.pickaxe.material_stat_types().is_some(),
            materials: &world.loaded.definitions,
            modifiers: &world.loaded.modifiers,
            item_tags_loaded: true,
            item: Some(&item),
            item_tags: None,
        };
        let built = crate::tool_build::build(
            &env,
            &[variant("test:wood"), variant("test:wood")],
            &[],
            2,
            1,
            1,
        )
        .unwrap();
        let crate::tool_build::Built::Success { tool: data, .. } = built else {
            panic!("expected a built tool");
        };
        let mut slot = ItemSlot::new(item.clone(), 1);
        slot.custom_data = data;
        tool_slot::sync_visuals(&mut slot, &env).unwrap();
        if damage > 0 {
            tool_slot::damage_slot(&mut slot, &env, damage, true, true, None, &mut || 1.0).unwrap();
        }
        slot
    }

    fn kit(material: &str, count: u32) -> ItemSlot {
        let mut slot = ItemSlot::new(id(REPAIR_KIT), count);
        let mut data = NbtCompound::new();
        data.insert("Material".into(), Nbt::String(material.to_owned()));
        slot.custom_data = data;
        slot
    }

    fn definition(world: &World) -> impl Fn(&Identifier) -> Option<Data> + '_ {
        |item: &Identifier| (item == &id("tconstruct:pickaxe")).then(|| world.pickaxe.clone())
    }
    fn in_tag(item: &Identifier, tag: &Identifier) -> bool {
        item == &id("tconstruct:pickaxe")
            && (tag == &id(DURABILITY_TAG) || tag == &id(MODIFIABLE_TAG))
    }

    fn grid<'a>(cells: &'a [Option<StackView<'a>>]) -> GridFacts<'a> {
        GridFacts { width: cells.len(), height: 1, left: 0, top: 0, cells }
    }
    fn view(slot: &ItemSlot) -> StackView<'_> {
        StackView {
            id: Box::leak(slot.item.to_string().into_boxed_str()),
            count: slot.count,
            damage: slot.damage,
            custom_data: Some(Box::leak(Box::new(Nbt::Compound(slot.custom_data.clone())))),
        }
    }

    #[test]
    fn grid_repair_kit_repairs_a_damaged_tool_and_scales_by_the_kit_amount() {
        let world = world();
        let hurt = tool(&world, 30);
        let kit_slot = kit("test:wood", 1);
        let cells = [Some(view(&hurt)), Some(view(&kit_slot))];
        // Kit amount 3.0 = one full ingot ratio: the head's 40 durability.
        let result = grid_repair(&world.loaded, 3.0, &definition(&world), &in_tag, &grid(&cells))
            .expect("the pair repairs");
        assert_eq!(result.item, "tconstruct:pickaxe");
        assert_eq!(result.count, 1);
        assert_eq!(result.damage, 0, "40 repair covers the 30 damage");
        let data = result.custom_data.unwrap();
        assert_eq!(tool_stack::damage(&data), 0);
        // Kit amount 1.5 = half ratio: 20 repair, 10 damage remains.
        let result = grid_repair(&world.loaded, 1.5, &definition(&world), &in_tag, &grid(&cells))
            .expect("the pair repairs");
        assert_eq!(tool_stack::damage(&result.custom_data.unwrap()), 10);
    }

    #[test]
    fn grid_rejects_wrong_pairs_undamaged_tools_and_unknown_materials() {
        let world = world();
        let hurt = tool(&world, 10);
        let sound = tool(&world, 0);
        let kit_slot = kit("test:wood", 1);
        let unknown_kit = kit("tconstruct:unknown", 1);
        let dirt = ItemSlot::new(id("minecraft:dirt"), 1);
        let def = definition(&world);
        // Undamaged tool: no match (the original's matches requires damage).
        let cells = [Some(view(&sound)), Some(view(&kit_slot))];
        assert!(grid_repair(&world.loaded, 3.0, &def, &in_tag, &grid(&cells)).is_none());
        // A stray third item, a second tool, a second kit: no match.
        let cells = [Some(view(&hurt)), Some(view(&kit_slot)), Some(view(&dirt))];
        assert!(grid_repair(&world.loaded, 3.0, &def, &in_tag, &grid(&cells)).is_none());
        let cells = [Some(view(&hurt)), Some(view(&hurt)), Some(view(&kit_slot))];
        assert!(grid_repair(&world.loaded, 3.0, &def, &in_tag, &grid(&cells)).is_none());
        let cells = [Some(view(&hurt)), Some(view(&kit_slot)), Some(view(&kit_slot))];
        assert!(grid_repair(&world.loaded, 3.0, &def, &in_tag, &grid(&cells)).is_none());
        // Kit of an unknown material, and a material that cannot repair: no match.
        let cells = [Some(view(&hurt)), Some(view(&unknown_kit))];
        assert!(grid_repair(&world.loaded, 3.0, &def, &in_tag, &grid(&cells)).is_none());
        let other_kit = kit("test:other", 1);
        let cells = [Some(view(&hurt)), Some(view(&other_kit))];
        assert!(grid_repair(&world.loaded, 3.0, &def, &in_tag, &grid(&cells)).is_none());
        // Either order in the grid matches.
        let cells = [Some(view(&kit_slot)), Some(view(&hurt))];
        assert!(grid_repair(&world.loaded, 3.0, &def, &in_tag, &grid(&cells)).is_some());
    }

    #[test]
    fn stacked_click_is_consumed_for_modifiable_tools_and_repairs_when_it_can() {
        use StackedOnOtherDecision as Decision;
        let world = world();
        let def = definition(&world);
        let hurt = tool(&world, 10);
        let sound = tool(&world, 0);
        let kit_slot = kit("test:wood", 2);
        fn request<'a>(
            carried: &'a ItemSlot,
            clicked: Option<&'a ItemSlot>,
            secondary: bool,
            allow: bool,
        ) -> StackedOnOtherRequest<'a> {
            StackedOnOtherRequest { carried, clicked, player: 0, secondary, allow_modification: allow, random: 0.5 }
        }
        // A repairing click: tool fixed, one kit taken, saw sound queued.
        let Decision::Apply { clicked, carried, give: None, sound: played } = stacked_repair(
            &world.loaded, 3.0, &def, &in_tag, &request(&kit_slot, Some(&hurt), true, true),
        ) else {
            panic!("expected an applied repair");
        };
        assert_eq!(tool_stack::damage(&clicked.unwrap().custom_data), 0);
        assert_eq!(carried.map(|slot| slot.count), Some(1), "one of the two kits is taken");
        assert_eq!(played, Some(StackedSound { id: SAW_SOUND, volume: 1.0, pitch: 1.0 }));
        // The last kit is taken whole.
        let one_kit = kit("test:wood", 1);
        let Decision::Apply { carried, .. } = stacked_repair(
            &world.loaded, 3.0, &def, &in_tag, &request(&one_kit, Some(&hurt), true, true),
        ) else {
            panic!("expected an applied repair");
        };
        assert!(carried.is_none());
        // Undamaged tool and unrepairable material: consumed, no repair.
        assert!(matches!(
            stacked_repair(&world.loaded, 3.0, &def, &in_tag, &request(&kit_slot, Some(&sound), true, true)),
            Decision::Consume
        ));
        let other_kit = kit("test:other", 1);
        assert!(matches!(
            stacked_repair(&world.loaded, 3.0, &def, &in_tag, &request(&other_kit, Some(&hurt), true, true)),
            Decision::Consume
        ));
        // Primary clicks, modification-locked slots and non-tools pass through.
        assert!(matches!(
            stacked_repair(&world.loaded, 3.0, &def, &in_tag, &request(&kit_slot, Some(&hurt), false, true)),
            Decision::Pass
        ));
        assert!(matches!(
            stacked_repair(&world.loaded, 3.0, &def, &in_tag, &request(&kit_slot, Some(&hurt), true, false)),
            Decision::Pass
        ));
        let dirt = ItemSlot::new(id("minecraft:dirt"), 1);
        assert!(matches!(
            stacked_repair(&world.loaded, 3.0, &def, &in_tag, &request(&kit_slot, Some(&dirt), true, true)),
            Decision::Pass
        ));
        assert!(matches!(
            stacked_repair(&world.loaded, 3.0, &def, &in_tag, &request(&kit_slot, None, true, true)),
            Decision::Pass
        ));
    }
}
