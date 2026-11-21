//! The G2 gadgets' shipped content, checked against the original's data: the throwable and
//! piggybackpack declarations, the glow block, the carry effect, the one recipe the original
//! has for any of them, and their assets and lang entries.
use std::path::PathBuf;

fn content() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content")
}

fn read_json(path: PathBuf) -> serde_json::Value {
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn horizon(kind: &str, name: &str) -> serde_json::Value {
    read_json(content().join(format!("data/tconstruct/horizon/{kind}/{name}.json")))
}

#[test]
fn the_four_throwables_are_declared_as_snowballs_stacking_to_sixteen() {
    for name in ["glow_ball", "efln_ball", "flint_shuriken", "quartz_shuriken"] {
        let item = horizon("item", name);
        assert_eq!(item["template"], "minecraft:snowball", "{name}");
        assert_eq!(item["max_stack_size"], 16, "{name}: Item.Properties().stacksTo(16)");
    }
    // ShurikenItem.use puts the item on a 4-tick cooldown: 0.2 seconds of use_cooldown.
    for name in ["flint_shuriken", "quartz_shuriken"] {
        let item = horizon("item", name);
        assert_eq!(item["components"]["minecraft:use_cooldown"]["seconds"], 0.2, "{name}");
    }
    for name in ["glow_ball", "efln_ball"] {
        assert!(horizon("item", name).get("components").is_none(), "{name} has no cooldown in Java");
    }
}

#[test]
fn the_thrown_entity_types_match_the_rust_table_and_the_java_damage() {
    for thrown in &crate::throwables::THROWABLES {
        let entity = horizon("entity_type", thrown.name);
        assert_eq!(entity["template"], "minecraft:snowball", "{}", thrown.name);
        assert_eq!(entity["item"], format!("tconstruct:{}", thrown.name), "{}", thrown.name);
        assert_eq!(entity["damage"].as_f64(), Some(thrown.damage as f64), "{}", thrown.name);
    }
}

#[test]
fn the_piggybackpack_and_the_carry_effect_are_declared() {
    let pack = horizon("item", "piggy_backpack");
    assert_eq!(pack["template"], "minecraft:paper");
    assert_eq!(pack["max_stack_size"], 16, "PiggyBackPackItem stacksTo(16)");
    let carry = horizon("mob_effect", "carry");
    assert_eq!(carry["category"], "neutral", "MobEffectCategory.NEUTRAL");
    assert_eq!(carry["color"], 16777215, "TinkerEffect's default colour 0xffffff");
    assert!(crate::creative::GENERAL.contains(&"piggy_backpack"), "the general tab lists it, as TinkerGadgets.addTabItems does");
    for name in ["glow_ball", "efln_ball", "flint_shuriken", "quartz_shuriken"] {
        assert!(!crate::creative::GENERAL.contains(&name), "{name} is not in the original's tab");
    }
}

#[test]
fn the_piggybackpack_recipe_is_the_originals_sky_slime_casting() {
    // GadgetRecipeProvider: a saddle (consumed) cast in sky slime, FluidValues.SLIMEBALL * 4.
    let recipe = read_json(content().join("data/tconstruct/recipes/fluids/gadgets/piggy_backpack.json"));
    assert_eq!(recipe["type"], "tconstruct:casting_table");
    assert_eq!(recipe["cast"]["item"], "minecraft:saddle");
    assert_eq!(recipe["cast_consumed"], true);
    assert_eq!(recipe["fluid"]["tag"], "tconstruct:sky_slime");
    assert_eq!(recipe["fluid"]["amount"], 1000);
    assert_eq!(recipe["result"], "tconstruct:piggy_backpack");
}

#[test]
fn the_piggyback_blacklist_tag_ships_empty_as_generated() {
    // EntityTypeTagProvider creates the tag with no entries.
    let tag = read_json(content().join("data/tconstruct/tags/entity_type/piggybackpack_blacklist.json"));
    assert_eq!(tag["values"], serde_json::json!([]));
}

#[test]
fn the_glow_block_declaration_carries_glowblocks_numbers() {
    let glow = horizon("block", "glow");
    assert_eq!(glow["template"], "minecraft:torch");
    assert_eq!(glow["properties"]["light"], 14, "lightLevel 14");
    assert_eq!(glow["properties"]["hardness"], 0.0);
    assert_eq!(glow["properties"]["sounds"], "minecraft:white_wool", "SoundType.WOOL");
    assert_eq!(glow["properties"]["map_color"], 0, "MapColor.NONE");
    assert_eq!(glow["properties"]["occludes"], false, "noOcclusion");
    assert_eq!(glow["item"], true, "registered with a block item");
    assert_eq!(glow["default_state"]["facing"], "down", "the Java default state");
    assert_eq!(glow["state_properties"]["facing"]["values"], serde_json::json!(["down", "up", "north", "south", "west", "east"]));
    assert_eq!(glow["collision_shapes"]["default"], serde_json::json!([]), "getCollisionShape is empty");
    // GlowBlock.getShape: a 1/16 slab against the facing side.
    let shapes = &glow["shapes"];
    assert_eq!(shapes["property"], "facing");
    assert_eq!(shapes["values"]["down"], serde_json::json!([[0, 0, 0, 16, 1, 16]]));
    assert_eq!(shapes["values"]["up"], serde_json::json!([[0, 15, 0, 16, 16, 16]]));
    assert_eq!(shapes["values"]["north"], serde_json::json!([[0, 0, 0, 16, 16, 1]]));
    assert_eq!(shapes["values"]["south"], serde_json::json!([[0, 0, 15, 16, 16, 16]]));
    assert_eq!(shapes["values"]["west"], serde_json::json!([[0, 0, 0, 1, 16, 16]]));
    assert_eq!(shapes["values"]["east"], serde_json::json!([[15, 0, 0, 16, 16, 16]]));
    // BuiltInLootTables.EMPTY: no loot table ships for the glow block.
    assert!(!content().join("data/tconstruct/loot_table/blocks/glow.json").exists());
}

#[test]
fn the_gadget_assets_and_lang_entries_ship() {
    let assets = content().join("assets/tconstruct");
    for name in ["glow_ball", "efln_ball", "flint_shuriken", "quartz_shuriken", "piggy_backpack", "glow"] {
        let model = read_json(assets.join(format!("models/item/{name}.json")));
        assert!(model["textures"]["layer0"].as_str().is_some_and(|t| t.starts_with("tconstruct:")), "{name} model");
    }
    for texture in ["glowball", "efln", "flint_shuriken", "quartz_shuriken", "piggy_backpack"] {
        assert!(assets.join(format!("textures/item/gadgets/{texture}.png")).exists(), "{texture}.png");
    }
    assert!(assets.join("textures/block/glow.png").exists());
    assert!(assets.join("models/block/glow.json").exists());
    assert!(assets.join("blockstates/glow.json").exists());

    let lang = read_json(assets.join("lang/en_us.json"));
    for (key, text) in [
        ("item.tconstruct.glow_ball", "Glowball"),
        ("item.tconstruct.glow_ball.tooltip", "Not suited for consumption."),
        ("item.tconstruct.efln_ball", "EFLN"),
        ("item.tconstruct.quartz_shuriken", "Quartz Shuriken"),
        ("item.tconstruct.flint_shuriken", "Flint Shuriken"),
        ("item.tconstruct.piggy_backpack", "Piggybackpack"),
        ("item.tconstruct.piggy_backpack.tooltip", "Use on a mob to pick it up"),
        ("effect.tconstruct.carry", "Carrying"),
    ] {
        assert_eq!(lang[key], text, "{key}");
    }
}
