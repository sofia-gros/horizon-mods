//! The fancy frames and armor stands as shipped content: the declarations the importer writes from
//! TinkerGadgets' registrations, the creative tab order, the converted recipes and the language keys.
use serde_json::{json, Value};
use std::path::PathBuf;

fn content() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content")
}

fn read(path: PathBuf) -> Value {
    serde_json::from_slice(&std::fs::read(&path).unwrap()).expect("valid JSON")
}

const FRAMES: [&str; 6] = ["gold_item_frame", "reversed_gold_item_frame", "diamond_item_frame", "manyullyn_item_frame", "netherite_item_frame", "clear_item_frame"];
const STANDS: [&str; 4] = ["bamboo_armor_stand", "bone_armor_stand", "necrotic_bone_armor_stand", "clear_armor_stand"];

#[test]
fn the_frame_and_stand_items_are_declared_on_their_vanilla_templates() {
    // TinkerGadgets registers the frame items with plain item properties and the stand items with
    // stacksTo(16); both item classes copy their vanilla base item's placement, hence the templates.
    for name in FRAMES {
        assert_eq!(read(content().join(format!("data/tconstruct/horizon/item/{name}.json"))), json!({"template": "minecraft:item_frame"}), "{name}");
    }
    for name in STANDS {
        assert_eq!(read(content().join(format!("data/tconstruct/horizon/item/{name}.json"))), json!({"template": "minecraft:armor_stand", "max_stack_size": 16}), "{name}");
    }
}

#[test]
fn the_frames_and_stands_sit_in_the_general_tab_in_the_original_order() {
    // TinkerGadgets.addTabItems, called by the general tab (TinkerCommons.addTabItems) after its own
    // tables, books, food, glass, bars and platforms sections: bamboo stand, clear frame,
    // clear stand, bone stand, necrotic bone stand, then the gold, reversed gold, diamond, manyullyn and
    // netherite frames (punji before them and the cakes after them are not declared at this point).
    let general = crate::creative::GENERAL;
    let gadgets: Vec<&str> = general.iter().copied().filter(|name| FRAMES.contains(name) || STANDS.contains(name)).collect();
    assert_eq!(gadgets, ["bamboo_armor_stand", "clear_item_frame", "clear_armor_stand", "bone_armor_stand", "necrotic_bone_armor_stand",
        "gold_item_frame", "reversed_gold_item_frame", "diamond_item_frame", "manyullyn_item_frame", "netherite_item_frame"]);
    let pattern = general.iter().position(|name| *name == "pattern").unwrap();
    let soul = general.iter().position(|name| *name == "soul_glass").unwrap();
    let first = general.iter().position(|name| *name == "bamboo_armor_stand").unwrap();
    assert!(pattern < soul && soul < first, "the gadgets follow the pattern and the soul glass");
}

#[test]
fn the_gadget_recipes_whose_items_exist_converted() {
    let recipes = content().join("data/tconstruct/recipe/gadgets");
    let bamboo = read(recipes.join("armor_stands/bamboo.json"));
    assert_eq!(bamboo["type"], "minecraft:crafting_shaped");
    assert_eq!(bamboo["key"], json!({"B": "tconstruct:pattern", "S": "minecraft:bamboo"}));
    assert_eq!(bamboo["pattern"], json!(["SSS", " S ", "SBS"]));
    assert_eq!(bamboo["result"]["id"], "tconstruct:bamboo_armor_stand");
    // The bone stand's sticks are the port's shipped tconstruct:bones tag (just minecraft:bone), kept as a
    // tag ingredient like the table recipes' vanilla-holding tags.
    let bone = read(recipes.join("armor_stands/bone.json"));
    assert_eq!(bone["key"], json!({"B": "tconstruct:seared_paver_slab", "S": "#tconstruct:bones"}));
    assert_eq!(bone["result"]["id"], "tconstruct:bone_armor_stand");
    // The reversed gold frame converts both ways with a redstone torch (group tconstruct:reverse_fancy_item_frame).
    let reversed = read(recipes.join("fancy_frame/reversed_gold.json"));
    assert_eq!(reversed["type"], "minecraft:crafting_shapeless");
    assert_eq!(reversed["ingredients"], json!(["tconstruct:gold_item_frame", "minecraft:redstone_torch"]));
    assert_eq!(reversed["result"]["id"], "tconstruct:reversed_gold_item_frame");
    let back = read(recipes.join("fancy_frame/reversed_reversed_gold.json"));
    assert_eq!(back["ingredients"], json!(["tconstruct:reversed_gold_item_frame", "minecraft:redstone_torch"]));
    assert_eq!(back["result"]["id"], "tconstruct:gold_item_frame");
    // The necrotic bone stand converted once the blazewood slab and the necrotic bone itself were
    // declared: its sticks are the bone (a one-item list, as the original ships it), its base the slab.
    let necrotic = read(recipes.join("armor_stands/necrotic_bone.json"));
    assert_eq!(necrotic["type"], "minecraft:crafting_shaped");
    assert_eq!(necrotic["key"], json!({"B": "tconstruct:blazewood_slab", "S": ["tconstruct:necrotic_bone"]}));
    assert_eq!(necrotic["pattern"], json!(["SSS", " S ", "SBS"]));
    assert_eq!(necrotic["result"]["id"], "tconstruct:necrotic_bone_armor_stand");
    // Of the seven that waited, six converted once the clear glass family and the obsidian pane were
    // declared (the clear stand and frame, and the gold, diamond, manyullyn and netherite frames),
    // and the necrotic bone stand followed once its items landed: nothing is left waiting.
    for converted in ["armor_stands/clear.json", "frame/gold.json", "frame/diamond.json", "frame/manyullyn.json", "frame/netherite.json", "fancy_frame/clear.json", "armor_stands/necrotic_bone.json"] {
        assert!(recipes.join(converted).exists(), "{converted} converted with its items declared");
    }
}

#[test]
fn the_language_file_names_every_frame_and_stand_and_its_tooltip() {
    let lang = read(content().join("assets/tconstruct/lang/en_us.json"));
    for name in FRAMES.iter().chain(STANDS.iter()) {
        assert!(lang.get(format!("item.tconstruct.{name}")).is_some(), "item.tconstruct.{name}");
        assert!(lang.get(format!("item.tconstruct.{name}.tooltip")).is_some(), "item.tconstruct.{name}.tooltip (the line the tooltip callback adds)");
    }
    assert!(lang.get("entity.tconstruct.item_frame").is_some(), "the frame entity's name");
    assert!(lang.get("entity.tconstruct.armor_stand").is_some(), "the stand entity's name");
}
