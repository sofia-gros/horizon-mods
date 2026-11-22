//! Config.java's common and client settings (SlimeKnights, MIT): every setting name, TOML section,
//! default and range matches the original's ForgeConfigSpec definitions.
//!
//! Consumers, per setting (the original's read site -> this port):
//! - The ten booleans in SETTINGS are the ConfigEnabledCondition props; conditions.rs evaluates
//!   them from the parsed map exactly as the original's `tconstruct:config` conditions do
//!   (`slime_recipe_fix` deliberately reads the glassRecipeFix value, as ConfigEnabledCondition
//!   registers it). Four props gate shipped data the port itself loads, so they are live:
//!   cheaper_netherite_alloy (the netherite alloy recipe), wither_bone_drop (the wither bone
//!   loot modifier), allow_ingotless_alloys (the ingotless alloy recipes) and
//!   force_integration_materials (integration material definitions and fluid effects).
//!   The other props' only original data users are files the host loads itself, whose
//!   conditions cannot name a config value: gravel_to_flint, glass_recipe_fix and
//!   slime_recipe_fix are recipe-level conditions on `recipe/` files, spawn_with_book is the
//!   conditional starting-book advancement, and slimy_loot_chests conditions Mantle loot
//!   injectors (no injector data ships, and PlutoLib has no loot-table injection surface).
//!   The host's recipe/advancement condition vocabulary is the closed neoforge set (an
//!   unknown type never loads) and PlutoLib has no seam into host data loading, so those
//!   files load unconditionally here; the props stay parsed and evaluable for any data the
//!   port itself loads.
//! - repairKitAmount: RepairKitItem.getRepairAmount and CraftingTableRepairKitRecipe ->
//!   station_menu.rs reads `repair_kit_amount()`.
//! - The four ore rates (melter/smeltery/foundry/foundry byproduct): MelterBlockEntity,
//!   SmelteryBlockEntity and FoundryBlockEntity build their melting inventories with them ->
//!   melter.rs and smeltery_menu.rs read `ore_rates()` at each melt, as the original reads
//!   `.get()` per use.
//! - toolTweaks (extendFireProtectionSlots, extendBlastProtectionSlots): TinkerTools runs the
//!   actions at construction to widen the vanilla FIRE_PROTECTION/BLAST_PROTECTION enchantments'
//!   applicable-slot arrays. Enchantments are host data here and PlutoLib has no mutable
//!   enchantment definitions, so the actions have no equivalent; the settings are parsed only.
//! - syncKnockbackResistance: TinkerAttributes.commonSetup marks the vanilla knockback
//!   resistance attribute syncable. PlutoLib exposes no switch for a vanilla attribute's
//!   syncing, so nothing reads the value yet (it is also a condition prop, above).
//! - toolInventorySync: ToolInventoryCapability.tryOpenContainer writes the sync type (and for
//!   FULL_STACK the whole stack) into the tool inventory's open packet. PlutoLib menus sync
//!   their own state, so the port's tool inventory has no per-open payload to choose.
//! - headDrops: WorldEvents.creeperKill drops a head when a charged creeper kills the matching
//!   mob -> world_blocks.rs registers creeper_head_drop on the drops callback and reads
//!   `head_drop()` per kill, after the native charged-creeper and spent-skull checks.
//! - wandererAncientToolWeight: WorldEvents.wanderingTrades adds that many copies of the
//!   ancient-tool listing to the wandering trader's rare trades -> trader.rs registers the
//!   listing with TradeOfferHelper at level 2; its provider reads
//!   `wanderer_ancient_tool_weight()` per pool snapshot.
//! - dropDragonScales: ToolEvents drops dragonScale items when a player-caused explosion
//!   damages an ender dragon. The port's damage hooks do not report that event shape.
//! - allowMonsterMeleeModifiers: ToolEvents gates the MONSTER_MELEE_DAMAGE and
//!   MONSTER_MELEE_HIT hooks on it. Those hooks are bound in modifier_modules.rs, but the
//!   port's melee dispatch (melee_hook.rs) only reports player attacks, so nothing reads
//!   the setting yet.
//! - disableSideInventoryWhitelist: TabbedContainerMenu.isUsable lets table screens show
//!   neighbouring inventories whose block entity type is in `tconstruct:side_inventories`
//!   (or any, with this on). The port's station menus have no side-inventory tabs.
//! - quickApplyToolModifiersSurvival: CreativeSlotItem.canApply (used by ModifierCrystalItem
//!   too) gates applying slot/crystal items onto a tool inside the player inventory. The
//!   port applies crystals at the tinker station only; the inventory path is not ported.
//! - debugLogResourceValues: JsonUtils.debugLogResourceValues gates value logging inside
//!   Mantle's JSON field readers. The port's loaders read serde_json directly; no equivalent.
//! - logInvalidToolStack: ToolStack.from logs (warning or stacktrace) when a stack is built
//!   from a non-modifiable item. The port has no ToolStack.from; its callers gate on the
//!   tool's materials tag instead.
//! - chemthrowerShotValue: read only by the Immersive Engineering plugin; this port has no
//!   IE integration by design, so the value is parsed and kept for the file format.
//! - Client settings: tankFluidModel switches tank fluids between a baked dynamic model
//!   (TankModel, ITankBlockEntity.useModelFluid) and the block-entity renderer; the port
//!   only has the renderer (rendering.rs). logMissingMaterialTextures and
//!   logMissingModifierTextures gate logging in the original's model bakers
//!   (MaterialModel/ToolModel/DynamicTextureLoader and friends), which the port's generated
//!   textures replace. maxSmelteryItemQuads caps HeatingStructureBlockEntityRenderer's item
//!   models by quad count; the port renders smeltery items as items, not counted model quads.
//!   modifiersIDsInAdvancedTooltips gates modifier IDs in TooltipUtil/ToolPartItem advanced
//!   tooltips; the port's tooltips have no advanced variant. The JEI settings are read only
//!   by JEIPlugin (and showToolInModifiers by IDisplayModifierRecipe's display); there is no
//!   JEI in this host. renderShieldSlotItem, renderSleevesItem and the itemFrame/minimap
//!   settings are read only by ModifierClientEvents' HUD rendering for the shield-slot,
//!   sleeves, framed and map modifiers, none of which render on the client here.
use pluto_lib::{ConfigEvents, ConfigKind, ConfigRegistry, ModConfig};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, RwLock};
use crate::fluid_values::OreRate;

// Source: common/config/Config.java and common/json/ConfigEnabledCondition.java.
const SETTINGS: &[(&str, &str, &str, bool)] = &[
    ("spawn_with_book", "gameplay", "shouldSpawnWithTinkersBook", true),
    ("sync_knockback_resistance", "gameplay", "syncKnockbackResistance", true),
    ("gravel_to_flint", "recipes", "addGravelToFlintRecipe", true),
    ("cheaper_netherite_alloy", "recipes", "cheaperNetheriteAlloy", true),
    ("wither_bone_drop", "recipes", "witherBoneDrop", true),
    ("glass_recipe_fix", "recipes", "glassRecipeFix", true),
    // The original condition deliberately references glassRecipeFix for BOTH names.
    ("slime_recipe_fix", "recipes", "glassRecipeFix", true),
    ("allow_ingotless_alloys", "compatability", "allowIngotlessAlloys", true),
    ("force_integration_materials", "debug", "forceIntegrationMaterials", false),
    ("slimy_loot_chests", "loot", "slimy_loot", true),
];

/// Config.repairKitAmount: durability of a repair kit in ingots, 2 by default, 0..=32767.
const REPAIR_KIT_AMOUNT: (&str, &str, f64) = ("gameplay", "repairKitAmount", 2.0);
static REPAIR_KIT: Mutex<Option<Result<f32, String>>> = Mutex::new(None);

fn parse_repair_kit_amount(text: &str) -> Result<f32, String> {
    let root: toml::Table = toml::from_str(text).map_err(|e| format!("invalid Tinkers common config: {e}"))?;
    let (section, key, default) = REPAIR_KIT_AMOUNT;
    let value = match root.get(section) {
        None => default,
        Some(table) => match table.as_table().ok_or_else(|| format!("{section} must be a table"))?.get(key) {
            None => default,
            Some(toml::Value::Float(value)) => *value,
            // TOML integers are accepted for a double setting, as Forge's config coerces them.
            Some(toml::Value::Integer(value)) => *value as f64,
            Some(_) => return Err(format!("{section}.{key} must be a number")),
        },
    };
    if !(0.0..=f64::from(i16::MAX)).contains(&value) { return Err(format!("{section}.{key} must be within 0..=32767")); }
    Ok(value as f32)
}

/// The loaded repair kit amount; an error (never the default) when the config is missing or invalid.
pub(crate) fn repair_kit_amount() -> Result<f32, String> {
    REPAIR_KIT.lock().expect("repair kit lock poisoned").clone().unwrap_or_else(|| Err("Tinkers common config has not loaded".into()))
}

pub(crate) fn known(name: &str) -> bool {
    name == "wither_bone_conversion" || SETTINGS.iter().any(|(condition, _, _, _)| *condition == name)
}

/// Config.ToolSyncType: how opening a tool inventory syncs the tool stack to the client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ToolSyncType { FullStack, Minimal, Disabled }

/// Config.LogInvalidToolStack: how building a tool stack from a non-modifiable item is logged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LogInvalidToolStack { Stacktrace, Warning, Ignored }

/// Orientation2D (library/utils): one of the 8 screen anchor positions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Orientation2D { TopLeft, Top, TopRight, Left, Right, BottomLeft, Bottom, BottomRight }

fn parse_tool_sync_type(text: &str) -> Option<ToolSyncType> {
    match text.to_ascii_uppercase().as_str() {
        "FULL_STACK" => Some(ToolSyncType::FullStack),
        "MINIMAL" => Some(ToolSyncType::Minimal),
        "DISABLED" => Some(ToolSyncType::Disabled),
        _ => None,
    }
}

fn parse_log_invalid_tool_stack(text: &str) -> Option<LogInvalidToolStack> {
    match text.to_ascii_uppercase().as_str() {
        "STACKTRACE" => Some(LogInvalidToolStack::Stacktrace),
        "WARNING" => Some(LogInvalidToolStack::Warning),
        "IGNORED" => Some(LogInvalidToolStack::Ignored),
        _ => None,
    }
}

fn parse_orientation(text: &str) -> Option<Orientation2D> {
    match text.to_ascii_uppercase().as_str() {
        "TOP_LEFT" => Some(Orientation2D::TopLeft),
        "TOP" => Some(Orientation2D::Top),
        "TOP_RIGHT" => Some(Orientation2D::TopRight),
        "LEFT" => Some(Orientation2D::Left),
        "RIGHT" => Some(Orientation2D::Right),
        "BOTTOM_LEFT" => Some(Orientation2D::BottomLeft),
        "BOTTOM" => Some(Orientation2D::Bottom),
        "BOTTOM_RIGHT" => Some(Orientation2D::BottomRight),
        _ => None,
    }
}

/// TinkerHeadType.getSerializedName values, in enum order: the keys of the `recipes.heads` section.
pub(crate) const HEAD_TYPES: [&str; 9] = [
    "blaze", "enderman", "stray", "husk", "drowned", "spider", "cave_spider", "piglin_brute", "zombified_piglin",
];

/// The four Config.OreRate values melting reads (melter, smeltery, foundry, foundry byproducts).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OreRates {
    pub melter: OreRate,
    pub smeltery: OreRate,
    pub foundry: OreRate,
    pub foundry_byproduct: OreRate,
}
impl Default for OreRates {
    fn default() -> Self {
        Self { melter: OreRate::MELTER, smeltery: OreRate::SMELTERY, foundry: OreRate::FOUNDRY, foundry_byproduct: OreRate::FOUNDRY_BYPRODUCT }
    }
}

/// Config.Common, field for field. Defaults are the original's.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Common {
    // gameplay
    pub should_spawn_with_tinkers_book: bool,
    pub extend_fire_protection_slots: bool,
    pub extend_blast_protection_slots: bool,
    pub sync_knockback_resistance: bool,
    pub tool_inventory_sync: ToolSyncType,
    pub repair_kit_amount: f32,
    // recipes
    pub add_gravel_to_flint_recipe: bool,
    pub cheaper_netherite_alloy: bool,
    pub wither_bone_drop: bool,
    pub slime_recipe_fix: bool,
    pub glass_recipe_fix: bool,
    pub melter_ore_rate: OreRate,
    pub smeltery_ore_rate: OreRate,
    pub foundry_ore_rate: OreRate,
    pub foundry_byproduct_rate: OreRate,
    /// recipes.heads, parallel to HEAD_TYPES.
    pub head_drops: [bool; 9],
    // loot
    pub slimy_loot_chests: bool,
    pub drop_dragon_scales: bool,
    pub wanderer_ancient_tool_weight: i32,
    // compatability
    pub allow_ingotless_alloys: bool,
    pub chemthrower_shot_value: f64,
    pub allow_monster_melee_modifiers: bool,
    // debug
    pub force_integration_materials: bool,
    pub disable_side_inventory_whitelist: bool,
    pub quick_apply_tool_modifiers_survival: bool,
    pub debug_log_resource_values: bool,
    pub log_invalid_tool_stack: LogInvalidToolStack,
}
impl Default for Common {
    fn default() -> Self {
        Self {
            should_spawn_with_tinkers_book: true,
            extend_fire_protection_slots: true,
            extend_blast_protection_slots: true,
            sync_knockback_resistance: true,
            tool_inventory_sync: ToolSyncType::Minimal,
            repair_kit_amount: 2.0,
            add_gravel_to_flint_recipe: true,
            cheaper_netherite_alloy: true,
            wither_bone_drop: true,
            slime_recipe_fix: true,
            glass_recipe_fix: true,
            melter_ore_rate: OreRate::MELTER,
            smeltery_ore_rate: OreRate::SMELTERY,
            foundry_ore_rate: OreRate::FOUNDRY,
            foundry_byproduct_rate: OreRate::FOUNDRY_BYPRODUCT,
            head_drops: [true; 9],
            slimy_loot_chests: true,
            drop_dragon_scales: true,
            wanderer_ancient_tool_weight: 6,
            allow_ingotless_alloys: true,
            chemthrower_shot_value: 1.25,
            allow_monster_melee_modifiers: true,
            force_integration_materials: false,
            disable_side_inventory_whitelist: false,
            quick_apply_tool_modifiers_survival: false,
            debug_log_resource_values: false,
            log_invalid_tool_stack: LogInvalidToolStack::Warning,
        }
    }
}
impl Common {
    /// Config.Common.headDrops for a TinkerHeadType serialized name; None for a name with no head.
    pub(crate) fn head_drop(&self, serialized_name: &str) -> Option<bool> {
        HEAD_TYPES.iter().position(|name| *name == serialized_name).map(|index| self.head_drops[index])
    }
    pub(crate) fn ore_rates(&self) -> OreRates {
        OreRates { melter: self.melter_ore_rate, smeltery: self.smeltery_ore_rate, foundry: self.foundry_ore_rate, foundry_byproduct: self.foundry_byproduct_rate }
    }
}

/// Config.Client, field for field. Defaults are the original's.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Client {
    pub tank_fluid_model: bool,
    pub log_missing_material_textures: bool,
    pub log_missing_modifier_textures: bool,
    // jei
    pub show_modifiers_in_jei: bool,
    pub show_only_tool_material: String,
    pub show_only_part_material: String,
    pub show_all_table_variants: bool,
    pub show_all_anvil_variants: bool,
    pub show_all_smeltery_variants: bool,
    pub show_filled_fluid_tanks: bool,
    pub show_potion_fluid_in_jei: bool,
    pub show_tool_in_modifiers: bool,
    // client
    pub max_smeltery_item_quads: i32,
    pub modifiers_ids_in_advanced_tooltips: bool,
    // modifiers
    pub render_shield_slot_item: bool,
    pub render_sleeves_item: bool,
    // modifiers.itemFrame
    pub render_item_frame: bool,
    pub item_frame_x_offset: i32,
    pub item_frame_y_offset: i32,
    pub item_frame_location: Orientation2D,
    pub items_per_row: i32,
    // modifiers.minimap
    pub map_x_offset: i32,
    pub map_y_offset: i32,
    pub map_scale: f64,
    pub map_location: Orientation2D,
}
impl Default for Client {
    fn default() -> Self {
        Self {
            tank_fluid_model: false,
            log_missing_material_textures: false,
            log_missing_modifier_textures: false,
            show_modifiers_in_jei: true,
            show_only_tool_material: String::new(),
            show_only_part_material: String::new(),
            show_all_table_variants: false,
            show_all_anvil_variants: true,
            show_all_smeltery_variants: false,
            show_filled_fluid_tanks: false,
            show_potion_fluid_in_jei: true,
            show_tool_in_modifiers: true,
            max_smeltery_item_quads: 3500,
            modifiers_ids_in_advanced_tooltips: true,
            render_shield_slot_item: true,
            render_sleeves_item: true,
            render_item_frame: true,
            item_frame_x_offset: 0,
            item_frame_y_offset: 0,
            item_frame_location: Orientation2D::TopLeft,
            items_per_row: 5,
            map_x_offset: 0,
            map_y_offset: 0,
            map_scale: 0.75,
            map_location: Orientation2D::TopLeft,
        }
    }
}

/// A position inside a config file's TOML: a missing section reads as all defaults (the same
/// stance as `parse` below), a section of the wrong type is an error.
struct Reader<'a> {
    table: Option<&'a toml::Table>,
    path: String,
}
impl<'a> Reader<'a> {
    fn section(&self, name: &str) -> Result<Reader<'a>, String> {
        let path = if self.path.is_empty() { name.to_owned() } else { format!("{}.{}", self.path, name) };
        let Some(table) = self.table else { return Ok(Reader { table: None, path }); };
        match table.get(name) {
            None => Ok(Reader { table: None, path }),
            Some(value) => Ok(Reader { table: Some(value.as_table().ok_or_else(|| format!("{path} must be a table"))?), path }),
        }
    }
    fn value(&self, key: &str) -> Option<&'a toml::Value> { self.table.and_then(|table| table.get(key)) }
    fn bool(&self, key: &str, default: bool) -> Result<bool, String> {
        match self.value(key) {
            None => Ok(default),
            Some(value) => value.as_bool().ok_or_else(|| format!("{}.{key} must be boolean", self.path)),
        }
    }
    fn int(&self, key: &str, default: i32, min: i32, max: i32) -> Result<i32, String> {
        match self.value(key) {
            None => Ok(default),
            Some(value) => {
                let value = value.as_integer().ok_or_else(|| format!("{}.{key} must be an integer", self.path))?;
                if !(i64::from(min)..=i64::from(max)).contains(&value) {
                    return Err(format!("{}.{key} must be within {min}..={max}", self.path));
                }
                Ok(value as i32)
            }
        }
    }
    fn float(&self, key: &str, default: f64, min: f64, max: f64) -> Result<f64, String> {
        match self.value(key) {
            None => Ok(default),
            Some(value) => {
                // TOML integers are accepted for a double setting, as Forge's config coerces them.
                let value = match value {
                    toml::Value::Float(value) => *value,
                    toml::Value::Integer(value) => *value as f64,
                    _ => return Err(format!("{}.{key} must be a number", self.path)),
                };
                if !(min..=max).contains(&value) { return Err(format!("{}.{key} must be within {min}..={max}", self.path)); }
                Ok(value)
            }
        }
    }
    fn string(&self, key: &str, default: &str) -> Result<String, String> {
        match self.value(key) {
            None => Ok(default.to_owned()),
            Some(value) => Ok(value.as_str().ok_or_else(|| format!("{}.{key} must be a string", self.path))?.to_owned()),
        }
    }
    fn enumeration<T: Copy>(&self, key: &str, default: T, parse: fn(&str) -> Option<T>) -> Result<T, String> {
        match self.value(key) {
            None => Ok(default),
            Some(value) => {
                let text = value.as_str().ok_or_else(|| format!("{}.{key} must be a string", self.path))?;
                parse(text).ok_or_else(|| format!("{}.{key} has unknown value {text}", self.path))
            }
        }
    }
}

fn ore_rate(section: &Reader, default: OreRate) -> Result<OreRate, String> {
    Ok(OreRate {
        nuggets_per_metal: section.int("nuggetsPerMetal", default.nuggets_per_metal, 1, 45)?,
        shards_per_gem: section.int("shardsPerGem", default.shards_per_gem, 1, 20)?,
    })
}

/// Parses the whole common config (Config.Common), not just the condition booleans.
pub(crate) fn parse_common(text: &str) -> Result<Common, String> {
    let root: toml::Table = toml::from_str(text).map_err(|e| format!("invalid Tinkers common config: {e}"))?;
    let root = Reader { table: Some(&root), path: String::new() };
    let defaults = Common::default();
    let gameplay = root.section("gameplay")?;
    let recipes = root.section("recipes")?;
    let ore_rates = recipes.section("ore_rates")?;
    let heads = recipes.section("heads")?;
    let mut head_drops = [true; 9];
    for (index, name) in HEAD_TYPES.iter().enumerate() { head_drops[index] = heads.bool(name, true)?; }
    let loot = root.section("loot")?;
    let compatability = root.section("compatability")?;
    let debug = root.section("debug")?;
    Ok(Common {
        should_spawn_with_tinkers_book: gameplay.bool("shouldSpawnWithTinkersBook", defaults.should_spawn_with_tinkers_book)?,
        extend_fire_protection_slots: gameplay.bool("extendFireProtectionSlots", defaults.extend_fire_protection_slots)?,
        extend_blast_protection_slots: gameplay.bool("extendBlastProtectionSlots", defaults.extend_blast_protection_slots)?,
        sync_knockback_resistance: gameplay.bool("syncKnockbackResistance", defaults.sync_knockback_resistance)?,
        tool_inventory_sync: gameplay.enumeration("toolInventorySync", defaults.tool_inventory_sync, parse_tool_sync_type)?,
        repair_kit_amount: gameplay.float("repairKitAmount", f64::from(defaults.repair_kit_amount), 0.0, f64::from(i16::MAX))? as f32,
        add_gravel_to_flint_recipe: recipes.bool("addGravelToFlintRecipe", defaults.add_gravel_to_flint_recipe)?,
        cheaper_netherite_alloy: recipes.bool("cheaperNetheriteAlloy", defaults.cheaper_netherite_alloy)?,
        wither_bone_drop: recipes.bool("witherBoneDrop", defaults.wither_bone_drop)?,
        slime_recipe_fix: recipes.bool("slimeRecipeFix", defaults.slime_recipe_fix)?,
        glass_recipe_fix: recipes.bool("glassRecipeFix", defaults.glass_recipe_fix)?,
        melter_ore_rate: ore_rate(&ore_rates.section("melter")?, defaults.melter_ore_rate)?,
        smeltery_ore_rate: ore_rate(&ore_rates.section("smeltery")?, defaults.smeltery_ore_rate)?,
        foundry_ore_rate: ore_rate(&ore_rates.section("foundry")?, defaults.foundry_ore_rate)?,
        foundry_byproduct_rate: ore_rate(&ore_rates.section("foundry_byproduct")?, defaults.foundry_byproduct_rate)?,
        head_drops,
        slimy_loot_chests: loot.bool("slimy_loot", defaults.slimy_loot_chests)?,
        drop_dragon_scales: loot.bool("drop_dragon_Scales", defaults.drop_dragon_scales)?,
        wanderer_ancient_tool_weight: loot.int("wanderer_ancient_tool_weight", defaults.wanderer_ancient_tool_weight, 0, 100)?,
        allow_ingotless_alloys: compatability.bool("allowIngotlessAlloys", defaults.allow_ingotless_alloys)?,
        chemthrower_shot_value: compatability.float("immersive_engineering_chemthrower_shot_value", defaults.chemthrower_shot_value, 0.0, f64::from(i32::MAX))?,
        allow_monster_melee_modifiers: compatability.bool("allowMonsterMeleeModifiers", defaults.allow_monster_melee_modifiers)?,
        force_integration_materials: debug.bool("forceIntegrationMaterials", defaults.force_integration_materials)?,
        disable_side_inventory_whitelist: debug.bool("disableSideInventoryWhitelist", defaults.disable_side_inventory_whitelist)?,
        quick_apply_tool_modifiers_survival: debug.bool("quickApplyToolModifiersSurvival", defaults.quick_apply_tool_modifiers_survival)?,
        debug_log_resource_values: debug.bool("debugLogResourceValues", defaults.debug_log_resource_values)?,
        log_invalid_tool_stack: debug.enumeration("logInvalidToolStack", defaults.log_invalid_tool_stack, parse_log_invalid_tool_stack)?,
    })
}

/// Parses the whole client config (Config.Client).
pub(crate) fn parse_client(text: &str) -> Result<Client, String> {
    let root: toml::Table = toml::from_str(text).map_err(|e| format!("invalid Tinkers client config: {e}"))?;
    let root = Reader { table: Some(&root), path: String::new() };
    let defaults = Client::default();
    let client = root.section("client")?;
    let jei = client.section("jei")?;
    let modifiers = client.section("modifiers")?;
    let frame = modifiers.section("itemFrame")?;
    let minimap = modifiers.section("minimap")?;
    // The original defines its HUD offsets with Short range limits.
    let short = (i32::from(i16::MIN), i32::from(i16::MAX));
    Ok(Client {
        tank_fluid_model: client.bool("tankFluidModel", defaults.tank_fluid_model)?,
        log_missing_material_textures: client.bool("logMissingMaterialTextures", defaults.log_missing_material_textures)?,
        log_missing_modifier_textures: client.bool("logMissingModifierTextures", defaults.log_missing_modifier_textures)?,
        show_modifiers_in_jei: jei.bool("showModifiers", defaults.show_modifiers_in_jei)?,
        show_only_tool_material: jei.string("showOnlyToolMaterial", &defaults.show_only_tool_material)?,
        show_only_part_material: jei.string("showOnlyPartMaterial", &defaults.show_only_part_material)?,
        show_all_table_variants: jei.bool("showAllTableVariants", defaults.show_all_table_variants)?,
        show_all_anvil_variants: jei.bool("showAllAnvilVariants", defaults.show_all_anvil_variants)?,
        show_all_smeltery_variants: jei.bool("showAllSmelteryVariants", defaults.show_all_smeltery_variants)?,
        show_filled_fluid_tanks: jei.bool("showFilledFluidTanks", defaults.show_filled_fluid_tanks)?,
        show_potion_fluid_in_jei: jei.bool("showPotionFluid", defaults.show_potion_fluid_in_jei)?,
        show_tool_in_modifiers: jei.bool("showToolInModifiers", defaults.show_tool_in_modifiers)?,
        max_smeltery_item_quads: client.int("maxSmelteryItemQuads", defaults.max_smeltery_item_quads, -1, i32::from(i16::MAX))?,
        modifiers_ids_in_advanced_tooltips: client.bool("modifiersIDsInAdvancedTooltips", defaults.modifiers_ids_in_advanced_tooltips)?,
        render_shield_slot_item: modifiers.bool("renderShieldSlotItem", defaults.render_shield_slot_item)?,
        render_sleeves_item: modifiers.bool("renderSleevesItem", defaults.render_sleeves_item)?,
        render_item_frame: frame.bool("render", defaults.render_item_frame)?,
        item_frame_x_offset: frame.int("xOffset", defaults.item_frame_x_offset, short.0, short.1)?,
        item_frame_y_offset: frame.int("yOffset", defaults.item_frame_y_offset, short.0, short.1)?,
        item_frame_location: frame.enumeration("location", defaults.item_frame_location, parse_orientation)?,
        items_per_row: frame.int("itemsPerRow", defaults.items_per_row, 0, 100)?,
        map_x_offset: minimap.int("xOffset", defaults.map_x_offset, short.0, short.1)?,
        map_y_offset: minimap.int("yOffset", defaults.map_y_offset, short.0, short.1)?,
        map_scale: minimap.float("scale", defaults.map_scale, 0.0, 100.0)?,
        map_location: minimap.enumeration("location", defaults.map_location, parse_orientation)?,
    })
}

static COMMON: Mutex<Option<Result<Common, String>>> = Mutex::new(None);
static CLIENT: Mutex<Option<Result<Client, String>>> = Mutex::new(None);

/// The loaded common config; an error (never the defaults) when it is missing or invalid,
/// the same stance as `repair_kit_amount`.
pub(crate) fn common() -> Result<Common, String> {
    COMMON.lock().expect("Tinkers common config lock poisoned").clone().unwrap_or_else(|| Err("Tinkers common config has not loaded".into()))
}

/// The loaded client config; an error (never the defaults) when it is missing or invalid.
pub(crate) fn client() -> Result<Client, String> {
    CLIENT.lock().expect("Tinkers client config lock poisoned").clone().unwrap_or_else(|| Err("Tinkers client config has not loaded".into()))
}

/// The ore rates melting reads. Forge substitutes the defaults for values it cannot read,
/// and melting is not a place that can report a config error to the player, so an unloaded
/// or invalid config melts at the defaults rather than not at all.
pub(crate) fn ore_rates() -> OreRates {
    common().map(|config| config.ore_rates()).unwrap_or_default()
}

/// Whether WorldEvents.creeperKill may drop the head of the TinkerHeadType serialized name; an unloaded
/// or invalid config drops at the defaults, the same stance as `ore_rates`.
pub(crate) fn head_drop(serialized_name: &str) -> bool {
    common().unwrap_or_default().head_drop(serialized_name).unwrap_or(false)
}

/// How many copies of the ancient-tool listing WorldEvents.wanderingTrades adds to the
/// wandering trader's rare pool; an unloaded or invalid config trades at the default,
/// the same stance as `ore_rates`.
pub(crate) fn wanderer_ancient_tool_weight() -> i32 {
    common().unwrap_or_default().wanderer_ancient_tool_weight
}

fn section_mut<'m>(root: &'m mut toml::map::Map<String, toml::Value>, path: &[&str]) -> &'m mut toml::map::Map<String, toml::Value> {
    let mut current = root;
    for name in path {
        let entry = current.entry((*name).to_owned()).or_insert_with(|| toml::Value::Table(Default::default()));
        current = entry.as_table_mut().expect("config section is a table");
    }
    current
}

pub(crate) fn default_text() -> String {
    let mut root = toml::map::Map::new();
    for &(_, section, key, default) in SETTINGS {
        let table = root.entry(section).or_insert_with(|| toml::Value::Table(Default::default()));
        table.as_table_mut().unwrap().insert(key.into(), toml::Value::Boolean(default));
    }
    let (section, key, default) = REPAIR_KIT_AMOUNT;
    root.entry(section).or_insert_with(|| toml::Value::Table(Default::default()))
        .as_table_mut().unwrap().insert(key.into(), toml::Value::Float(default));
    {
        let gameplay = section_mut(&mut root, &["gameplay"]);
        gameplay.insert("extendFireProtectionSlots".into(), toml::Value::Boolean(true));
        gameplay.insert("extendBlastProtectionSlots".into(), toml::Value::Boolean(true));
        gameplay.insert("toolInventorySync".into(), toml::Value::String("MINIMAL".into()));
    }
    for (name, rate) in [("melter", OreRate::MELTER), ("smeltery", OreRate::SMELTERY), ("foundry", OreRate::FOUNDRY), ("foundry_byproduct", OreRate::FOUNDRY_BYPRODUCT)] {
        let table = section_mut(&mut root, &["recipes", "ore_rates", name]);
        table.insert("nuggetsPerMetal".into(), toml::Value::Integer(i64::from(rate.nuggets_per_metal)));
        table.insert("shardsPerGem".into(), toml::Value::Integer(i64::from(rate.shards_per_gem)));
    }
    {
        let heads = section_mut(&mut root, &["recipes", "heads"]);
        for name in HEAD_TYPES { heads.insert(name.into(), toml::Value::Boolean(true)); }
        let loot = section_mut(&mut root, &["loot"]);
        loot.insert("drop_dragon_Scales".into(), toml::Value::Boolean(true));
        loot.insert("wanderer_ancient_tool_weight".into(), toml::Value::Integer(6));
        let compatability = section_mut(&mut root, &["compatability"]);
        compatability.insert("immersive_engineering_chemthrower_shot_value".into(), toml::Value::Float(1.25));
        compatability.insert("allowMonsterMeleeModifiers".into(), toml::Value::Boolean(true));
        let debug = section_mut(&mut root, &["debug"]);
        debug.insert("disableSideInventoryWhitelist".into(), toml::Value::Boolean(false));
        debug.insert("quickApplyToolModifiersSurvival".into(), toml::Value::Boolean(false));
        debug.insert("debugLogResourceValues".into(), toml::Value::Boolean(false));
        debug.insert("logInvalidToolStack".into(), toml::Value::String("WARNING".into()));
    }
    toml::to_string(&root).expect("config defaults serialize")
}

pub(crate) fn client_default_text() -> String {
    let mut root = toml::map::Map::new();
    {
        let client = section_mut(&mut root, &["client"]);
        client.insert("tankFluidModel".into(), toml::Value::Boolean(false));
        client.insert("logMissingMaterialTextures".into(), toml::Value::Boolean(false));
        client.insert("logMissingModifierTextures".into(), toml::Value::Boolean(false));
        client.insert("maxSmelteryItemQuads".into(), toml::Value::Integer(3500));
        client.insert("modifiersIDsInAdvancedTooltips".into(), toml::Value::Boolean(true));
    }
    {
        let jei = section_mut(&mut root, &["client", "jei"]);
        jei.insert("showModifiers".into(), toml::Value::Boolean(true));
        jei.insert("showOnlyToolMaterial".into(), toml::Value::String(String::new()));
        jei.insert("showOnlyPartMaterial".into(), toml::Value::String(String::new()));
        jei.insert("showAllTableVariants".into(), toml::Value::Boolean(false));
        jei.insert("showAllAnvilVariants".into(), toml::Value::Boolean(true));
        jei.insert("showAllSmelteryVariants".into(), toml::Value::Boolean(false));
        jei.insert("showFilledFluidTanks".into(), toml::Value::Boolean(false));
        jei.insert("showPotionFluid".into(), toml::Value::Boolean(true));
        jei.insert("showToolInModifiers".into(), toml::Value::Boolean(true));
    }
    {
        let modifiers = section_mut(&mut root, &["client", "modifiers"]);
        modifiers.insert("renderShieldSlotItem".into(), toml::Value::Boolean(true));
        modifiers.insert("renderSleevesItem".into(), toml::Value::Boolean(true));
    }
    {
        let frame = section_mut(&mut root, &["client", "modifiers", "itemFrame"]);
        frame.insert("render".into(), toml::Value::Boolean(true));
        frame.insert("xOffset".into(), toml::Value::Integer(0));
        frame.insert("yOffset".into(), toml::Value::Integer(0));
        frame.insert("location".into(), toml::Value::String("TOP_LEFT".into()));
        frame.insert("itemsPerRow".into(), toml::Value::Integer(5));
    }
    {
        let minimap = section_mut(&mut root, &["client", "modifiers", "minimap"]);
        minimap.insert("xOffset".into(), toml::Value::Integer(0));
        minimap.insert("yOffset".into(), toml::Value::Integer(0));
        minimap.insert("scale".into(), toml::Value::Float(0.75));
        minimap.insert("location".into(), toml::Value::String("TOP_LEFT".into()));
    }
    toml::to_string(&root).expect("client config defaults serialize")
}

pub(crate) fn parse(text: &str) -> Result<BTreeMap<String, bool>, String> {
    let root: toml::Table = toml::from_str(text).map_err(|e| format!("invalid Tinkers common config: {e}"))?;
    let mut values = BTreeMap::new();
    for &(condition, section, key, default) in SETTINGS {
        let value = match root.get(section) {
            None => default,
            Some(table) => match table.as_table().ok_or_else(|| format!("{section} must be a table"))?.get(key) {
                None => default,
                Some(value) => value.as_bool().ok_or_else(|| format!("{section}.{key} must be boolean"))?,
            },
        };
        values.insert(condition.into(), value);
    }
    values.insert("wither_bone_conversion".into(), false);
    Ok(values)
}

type State = Arc<RwLock<Result<BTreeMap<String, bool>, String>>>;
pub(crate) fn register() -> State {
    let state = Arc::new(RwLock::new(Err("Tinkers common config has not loaded".into())));
    ConfigRegistry::register("tconstruct", ConfigKind::Common, &default_text());
    for event in [ConfigEvents::LOADING, ConfigEvents::RELOADING] {
        let output = state.clone();
        event.register(move |config: &ModConfig| {
            if config.mod_id == "tconstruct" && config.kind == ConfigKind::Common {
                let parsed = parse(&config.text);
                if let Err(error) = &parsed { eprintln!("Tinkers: {error}"); }
                *REPAIR_KIT.lock().expect("repair kit lock poisoned") = Some(parse_repair_kit_amount(&config.text));
                *COMMON.lock().expect("Tinkers common config lock poisoned") = Some(parse_common(&config.text));
                *output.write().expect("Tinkers config lock poisoned") = parsed;
            }
        });
    }
    ConfigRegistry::register("tconstruct", ConfigKind::Client, &client_default_text());
    for event in [ConfigEvents::LOADING, ConfigEvents::RELOADING] {
        event.register(move |config: &ModConfig| {
            if config.mod_id == "tconstruct" && config.kind == ConfigKind::Client {
                let parsed = parse_client(&config.text);
                if let Err(error) = &parsed { eprintln!("Tinkers: {error}"); }
                *CLIENT.lock().expect("Tinkers client config lock poisoned") = Some(parsed);
            }
        });
    }
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_aliases_defaults_and_invalid_values() {
        let defaults = parse(&default_text()).unwrap();
        assert_eq!(parse_repair_kit_amount(&default_text()).unwrap(), 2.0, "the written default round-trips");
        assert!(!defaults["force_integration_materials"]);
        assert!(!defaults["wither_bone_conversion"]);
        let changed = parse("[recipes]\nglassRecipeFix = false\nslimeRecipeFix = true\n[debug]\nforceIntegrationMaterials = true\n").unwrap();
        assert!(!changed["glass_recipe_fix"] && !changed["slime_recipe_fix"]);
        assert!(changed["force_integration_materials"] && changed["spawn_with_book"]);
        assert!(parse("[debug]\nforceIntegrationMaterials = 'true'\n").is_err());
        assert!(parse("debug = false").is_err());
    }
    #[test]
    fn repair_kit_amount_defaults_to_two_and_rejects_out_of_range_or_non_numbers() {
        assert_eq!(parse_repair_kit_amount("").unwrap(), 2.0);
        assert_eq!(parse_repair_kit_amount("[gameplay]\nrepairKitAmount = 3.5\n").unwrap(), 3.5);
        assert_eq!(parse_repair_kit_amount("[gameplay]\nrepairKitAmount = 0\n").unwrap(), 0.0);
        for bad in ["[gameplay]\nrepairKitAmount = -1.0\n", "[gameplay]\nrepairKitAmount = 40000\n",
                    "[gameplay]\nrepairKitAmount = 'two'\n", "gameplay = 1"] {
            assert!(parse_repair_kit_amount(bad).is_err(), "{bad}");
        }
    }
    #[test]
    fn common_defaults_round_trip_and_changed_values_read() {
        assert_eq!(parse_common(&default_text()).unwrap(), Common::default(), "the written defaults parse back to the defaults");
        assert_eq!(parse_common("").unwrap(), Common::default(), "an empty file is all defaults");
        let changed = parse_common("[gameplay]\ntoolInventorySync = 'FULL_STACK'\nextendFireProtectionSlots = false\n\
            [recipes.ore_rates.melter]\nnuggetsPerMetal = 45\nshardsPerGem = 1\n\
            [recipes.heads]\nblaze = false\n[loot]\nwanderer_ancient_tool_weight = 0\ndrop_dragon_Scales = false\n\
            [compatability]\nimmersive_engineering_chemthrower_shot_value = 2\n\
            [debug]\nlogInvalidToolStack = 'stacktrace'\ndisableSideInventoryWhitelist = true\n").unwrap();
        assert_eq!(changed.tool_inventory_sync, ToolSyncType::FullStack);
        assert!(!changed.extend_fire_protection_slots && changed.extend_blast_protection_slots);
        assert_eq!(changed.melter_ore_rate, OreRate { nuggets_per_metal: 45, shards_per_gem: 1 });
        assert_eq!(changed.smeltery_ore_rate, OreRate::SMELTERY, "untouched sections keep their defaults");
        assert_eq!(changed.head_drop("blaze"), Some(false));
        assert_eq!(changed.head_drop("stray"), Some(true));
        assert_eq!(changed.head_drop("creeper"), None);
        assert_eq!(changed.wanderer_ancient_tool_weight, 0);
        assert!(!changed.drop_dragon_scales);
        assert_eq!(changed.chemthrower_shot_value, 2.0, "a TOML integer coerces to the double setting");
        assert_eq!(changed.log_invalid_tool_stack, LogInvalidToolStack::Stacktrace);
        assert!(changed.disable_side_inventory_whitelist);
        assert_eq!(changed.ore_rates().melter, changed.melter_ore_rate);
        assert_eq!(changed.ore_rates().foundry_byproduct, OreRate::FOUNDRY_BYPRODUCT);
    }
    #[test]
    fn common_rejects_out_of_range_mistyped_and_unknown_values() {
        for bad in ["[recipes.ore_rates.melter]\nnuggetsPerMetal = 46\n", "[recipes.ore_rates.melter]\nnuggetsPerMetal = 0\n",
                    "[recipes.ore_rates.foundry]\nshardsPerGem = 21\n", "[loot]\nwanderer_ancient_tool_weight = 101\n",
                    "[loot]\nwanderer_ancient_tool_weight = 1.5\n", "[gameplay]\ntoolInventorySync = 'SOMEWHAT'\n",
                    "[gameplay]\ntoolInventorySync = 3\n", "[compatability]\nimmersive_engineering_chemthrower_shot_value = -1\n",
                    "[recipes.heads]\nblaze = 1\n", "recipes = true"] {
            assert!(parse_common(bad).is_err(), "{bad}");
        }
    }
    #[test]
    fn client_defaults_round_trip_and_nested_sections_read() {
        assert_eq!(parse_client(&client_default_text()).unwrap(), Client::default(), "the written defaults parse back to the defaults");
        assert_eq!(parse_client("").unwrap(), Client::default(), "an empty file is all defaults");
        let changed = parse_client("[client]\ntankFluidModel = true\nmaxSmelteryItemQuads = -1\n\
            [client.jei]\nshowOnlyToolMaterial = 'tconstruct:steel'\nshowAllAnvilVariants = false\n\
            [client.modifiers.itemFrame]\nlocation = 'BOTTOM_RIGHT'\nitemsPerRow = 0\n\
            [client.modifiers.minimap]\nscale = 0\nxOffset = -32768\n").unwrap();
        assert!(changed.tank_fluid_model);
        assert_eq!(changed.max_smeltery_item_quads, -1, "the -1 (unlimited) end of the range is accepted");
        assert_eq!(changed.show_only_tool_material, "tconstruct:steel");
        assert!(!changed.show_all_anvil_variants && changed.show_modifiers_in_jei);
        assert_eq!(changed.item_frame_location, Orientation2D::BottomRight);
        assert_eq!(changed.items_per_row, 0);
        assert_eq!(changed.map_scale, 0.0);
        assert_eq!(changed.map_x_offset, -32768);
        for bad in ["[client]\nmaxSmelteryItemQuads = 32768\n", "[client]\nmaxSmelteryItemQuads = -2\n",
                    "[client.modifiers.minimap]\nscale = 101\n", "[client.modifiers.minimap]\nscale = 'big'\n",
                    "[client.modifiers.itemFrame]\nlocation = 'CENTER'\n", "[client.modifiers.itemFrame]\nxOffset = 32768\n",
                    "[client.jei]\nshowOnlyToolMaterial = 3\n", "client = 1"] {
            assert!(parse_client(bad).is_err(), "{bad}");
        }
    }
}
