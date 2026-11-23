//! ToolStack NBT persistence from SlimeKnights (MIT; see LICENSE).
//! Operates on a stack's custom-data compound with the original tic_* keys.
use crate::{
    materials::Definitions as Materials, modifier_entries::List, modifiers::Registry, tool_definitions::Data,
    tool_materials::MaterialList, tool_rebuild::{rebuild, Readiness, Rebuilt}, tool_stats::Stat,
};
use pluto_lib::{Identifier, Nbt, NbtCompound};

pub(crate) const TAG_MATERIALS: &str = "tic_materials";
pub(crate) const TAG_PERSISTENT: &str = "tic_persistent";
pub(crate) const TAG_UPGRADES: &str = "tic_upgrades";
pub(crate) const TAG_BROKEN: &str = "tic_broken";
pub(crate) const TAG_STATS: &str = "tic_stats";
pub(crate) const TAG_MULTIPLIERS: &str = "tic_multipliers";
pub(crate) const TAG_VOLATILE: &str = "tic_volatile_data";
pub(crate) const TAG_MODIFIERS: &str = "tic_modifiers";
pub(crate) const TAG_ADDED_ENCHANTMENTS: &str = "tic_added_enchantments";
pub(crate) const TAG_DAMAGE: &str = "Damage";
const TIER_KEY: &str = "tconstruct:harvest_tier";

/// ToolStack.isInitialized: only the presence of a compound under tic_stats counts.
pub(crate) fn is_initialized(nbt: &NbtCompound) -> bool { matches!(nbt.get(TAG_STATS), Some(Nbt::Compound(_))) }

fn boolean(nbt: &NbtCompound, key: &str) -> bool { nbt.get(key).and_then(Nbt::as_i64).is_some_and(|value| value != 0) }

/// MaterialNBT is empty for definitions without material slots, regardless of saved data.
pub(crate) fn materials(nbt: &NbtCompound, has_materials: bool) -> MaterialList {
    if has_materials { MaterialList::from_nbt(nbt.get(TAG_MATERIALS)) } else { MaterialList::default() }
}
/// setMaterialsRaw: an empty list removes the key; no rebuild.
pub(crate) fn set_materials_raw(nbt: &mut NbtCompound, materials: &MaterialList) {
    if materials.as_slice().is_empty() { nbt.remove(TAG_MATERIALS); } else { nbt.insert(TAG_MATERIALS.into(), materials.to_nbt()); }
}
pub(crate) fn upgrades(nbt: &NbtCompound) -> List { List::from_nbt(nbt.get(TAG_UPGRADES)) }

/// Everything a rebuild reads besides the stack itself.
pub(crate) struct Environment<'a> {
    pub definition: Option<&'a Data>,
    pub has_materials: bool,
    pub materials: &'a Materials,
    pub modifiers: &'a Registry,
    pub item_tags_loaded: bool,
    /// The tool's item and the item-tag snapshot, for tool predicates and stat support; None outside a stack.
    pub item: Option<&'a Identifier>,
    pub item_tags: Option<&'a pluto_lib::RegistryLookup>,
}

/// ToolStack.rebuildStats. Returns whether state was rebuilt; unready environments are a no-op.
pub(crate) fn rebuild_stats(nbt: &mut NbtCompound, env: &Environment) -> Result<bool, String> {
    let readiness = Readiness {
        definition: env.definition.is_some(), materials: env.materials.server_loaded,
        modifiers: env.modifiers.definitions_loaded, item_tags: env.item_tags_loaded,
    };
    let Some(definition) = env.definition else { return Ok(false); };
    let list = materials(nbt, env.has_materials);
    let saved = upgrades(nbt);
    let persistent = match nbt.get(TAG_PERSISTENT) { Some(Nbt::Compound(data)) => data.clone(), _ => NbtCompound::new() };
    let has_hook = |hook: &Identifier| definition.has_hook(hook);
    let hook_traits = |hook: &Identifier| definition.hook_traits(hook, &list, env.materials).unwrap_or_default();
    let tool = crate::tool_rebuild::Tool { item: env.item, item_tags: env.item_tags, persistent: &persistent, hook_traits: Some(&hook_traits), has_hook: Some(&has_hook), modifier_tags: Some(&env.modifiers.reverse_tags) };
    let Some(rebuilt) = rebuild(readiness, definition, &list, &saved, env.materials, env.modifiers, &tool)? else { return Ok(false); };
    apply(nbt, &rebuilt);
    apply_raw_data(nbt, env);
    // TankModule.validateFluid: a tank that shrank under its content clamps to the new capacity.
    crate::tool_tank::validate_clamp(nbt);
    Ok(true)
}

/// RawDataModifierHook over the rebuilt tool, run by the caller that owns the stack (see
/// tool_rebuild): the top-level keys another mod reads, rebuilt from scratch like the volatile
/// data, and EmbellishmentModule's migration of its stored variant through material redirects.
fn apply_raw_data(nbt: &mut NbtCompound, env: &Environment) {
    // The only writers of top-level keys are TheOneProbeModule's two (EmbellishmentModule's raw
    // data half writes the persistent data instead, below), so those are the keys cleared first.
    for key in ["theoneprobe", "theoneprobe_hand"] { nbt.remove(key); }
    let raw = with_tool_facts(nbt, env, |saved, facts| {
        let mut raw = NbtCompound::new();
        for (entry, modifier) in saved { modifier.apply_raw_data(entry, facts, &mut raw); }
        raw
    });
    for (key, value) in raw.iter() { nbt.insert(key.clone(), value.clone()); }
    // EmbellishmentModule.addRawData: the stored variant follows material redirects on build.
    let persistent = match nbt.get(TAG_PERSISTENT) { Some(Nbt::Compound(data)) => data.clone(), _ => NbtCompound::new() };
    let edits: Vec<(String, Nbt)> = saved_modifiers(nbt, env.modifiers).iter()
        .filter_map(|(_, modifier)| modifier.embellish_migration(&persistent, env.materials)).collect();
    if !edits.is_empty() {
        let mut persistent = persistent;
        for (key, value) in edits { persistent.insert(key, value); }
        nbt.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
    }
}

/// ToolStack.ensureHasData's material half: a tool holding fewer materials than it has stat
/// types is filled (MissingMaterialsToolHook) and the fill is written back; the caller then
/// rebuilds, as ensureHasData does. This runs on the verifyTag paths (a stack about to be
/// used), never inside rebuild_stats itself: the original fills on inventory tick, so a
/// station-built stack keeps exactly the materials it was built with until it is used.
/// Returns whether a fill was written. Like the original, nothing happens before the
/// material data is loaded.
pub(crate) fn ensure_materials(nbt: &mut NbtCompound, env: &Environment) -> bool {
    let Some(definition) = env.definition else { return false; };
    if !env.has_materials || !env.materials.server_loaded { return false; }
    let list = materials(nbt, env.has_materials);
    match definition.fill_materials(&list, env.materials, &mut || crate::tool_damage_hook::random()) {
        Some(filled) if filled != list => { set_materials_raw(nbt, &filled); true }
        _ => false,
    }
}

fn apply(nbt: &mut NbtCompound, rebuilt: &Rebuilt) {
    // setModifiers always writes a list, even when empty.
    nbt.insert(TAG_MODIFIERS.into(), Nbt::List(rebuilt.modifiers.iter().map(|(entry, _)| Nbt::Compound(entry.to_nbt())).collect()));
    // setVolatileModData removes an empty compound instead of storing it.
    if rebuilt.volatile.is_empty() { nbt.remove(TAG_VOLATILE); } else { nbt.insert(TAG_VOLATILE.into(), Nbt::Compound(rebuilt.volatile.clone())); }
    let mut stats = NbtCompound::new();
    for (stat, value) in &rebuilt.stats { stats.insert(format!("tconstruct:{}", stat.path()), Nbt::Float(*value)); }
    if let Some(tier) = &rebuilt.tier { stats.insert(TIER_KEY.into(), Nbt::String(tier.to_string())); }
    nbt.insert(TAG_STATS.into(), Nbt::Compound(stats));
    // setStats: lower the damage (and mark broken) when the new durability no longer holds it.
    let durability = durability(nbt);
    if damage_raw(nbt) >= durability { set_damage(nbt, durability); }
    let mut multipliers = NbtCompound::new();
    for (stat, value) in &rebuilt.multipliers { multipliers.insert(format!("tconstruct:{}", stat.path()), Nbt::Float(*value)); }
    if multipliers.is_empty() { nbt.remove(TAG_MULTIPLIERS); } else { nbt.insert(TAG_MULTIPLIERS.into(), Nbt::Compound(multipliers)); }
}

/// The tool's final modifier list as saved in tic_modifiers, each with its loaded definition;
/// entries with no loaded definition are not bound and are skipped.
pub(crate) fn saved_modifiers(nbt: &NbtCompound, registry: &Registry) -> Vec<(crate::modifier_entries::Entry, std::sync::Arc<crate::modifiers::Modifier>)> {
    List::from_nbt(nbt.get(TAG_MODIFIERS)).entries().iter()
        .filter_map(|entry| registry.get(&entry.id).map(|modifier| (entry.clone(), modifier.clone()))).collect()
}

fn with_tool_facts<R>(nbt: &NbtCompound, env: &Environment, run: impl FnOnce(&[(crate::modifier_entries::Entry, std::sync::Arc<crate::modifiers::Modifier>)], &crate::tool_predicate::Facts) -> R) -> R {
    let saved = saved_modifiers(nbt, env.modifiers);
    let persistent = match nbt.get(TAG_PERSISTENT) { Some(Nbt::Compound(data)) => data.clone(), _ => NbtCompound::new() };
    let list = materials(nbt, env.has_materials);
    let has_hook = |hook: &Identifier| env.definition.is_some_and(|data| data.has_hook(hook));
    let hook_traits = |hook: &Identifier| env.definition.and_then(|data| data.hook_traits(hook, &list, env.materials).ok()).unwrap_or_default();
    let tool = crate::tool_rebuild::Tool { item: env.item, item_tags: env.item_tags, persistent: &persistent, hook_traits: Some(&hook_traits), has_hook: Some(&has_hook), modifier_tags: Some(&env.modifiers.reverse_tags) };
    let upgrades = upgrades(nbt);
    crate::tool_rebuild::with_facts(&tool, &saved, &upgrades, |facts| run(&saved, facts))
}

/// BreakSpeedModifierHook over every modifier of the tool: each conditional mining speed module in
/// priority order (highest first, ties in modifier order), starting from `speed`.
pub(crate) fn break_speed(nbt: &NbtCompound, env: &Environment, context: &crate::mining_speed::Context, speed: f32) -> f32 {
    // Definition mining-speed modules compose ahead of the modifier modules below
    // (MiningSpeedToolHook runs definition modules before modifier hooks).
    let speed = match env.definition { Some(definition) => definition.modify_mining_speed(context, speed), None => speed };
    with_tool_facts(nbt, env, |saved, facts| {
        let mut modules: Vec<(i32, usize, &crate::modifier_entries::Entry, &crate::modifier_modules::Module)> = Vec::new();
        for (index, (entry, modifier)) in saved.iter().enumerate() {
            for (priority, module) in modifier.mining_modules() { modules.push((priority, index, entry, module)); }
        }
        modules.sort_by_key(|(priority, index, _, _)| (std::cmp::Reverse(*priority), *index));
        // The formula's entity variables read the digging player the event reported (the holder).
        let view = match &context.holder {
            Some(holder) => crate::mining_speed::ToolView::with(nbt, crate::entity_facts::Entities { attacker: Some(holder), target: None }),
            None => crate::mining_speed::ToolView::of(nbt),
        };
        modules.into_iter().fold(speed, |speed, (_, _, entry, module)| module.mining_speed(entry, facts, &view, context, speed))
    })
}

/// The enchantments the tool's modifiers give it (EnchantmentModifierHook), levels adding up per enchantment.
pub(crate) fn enchantments(nbt: &NbtCompound, env: &Environment) -> std::collections::BTreeMap<String, i32> {
    with_tool_facts(nbt, env, |saved, facts| {
        let mut levels: std::collections::BTreeMap<String, i32> = std::collections::BTreeMap::new();
        for (entry, modifier) in saved { for (name, level) in modifier.enchantments(entry, facts) { let total = levels.entry(name).or_insert(0); *total = total.saturating_add(level); } }
        levels
    })
}

/// The enchantments a player added to the stack (anvil, enchanting table), as the last slot sync
/// recorded them in the tool's data. The stack's component carries them next to the ones its
/// modifiers give, but a damage event reports no components, so this record is the only place a
/// reader without the slot can still count them.
pub(crate) fn added_enchantments(nbt: &NbtCompound) -> std::collections::BTreeMap<String, i32> {
    let mut levels = std::collections::BTreeMap::new();
    if let Some(Nbt::Compound(entries)) = nbt.get(TAG_ADDED_ENCHANTMENTS) {
        for (name, value) in entries { if let Some(level) = value.as_i64() { levels.insert(name.clone(), level as i32); } }
    }
    levels
}

/// Records the player-added enchantments (see `added_enchantments`); an empty set removes the key.
pub(crate) fn set_added_enchantments(nbt: &mut NbtCompound, levels: &std::collections::BTreeMap<String, i32>) {
    if levels.is_empty() { nbt.remove(TAG_ADDED_ENCHANTMENTS); return; }
    let mut entries = NbtCompound::new();
    for (name, level) in levels { entries.insert(name.clone(), Nbt::Int(*level)); }
    nbt.insert(TAG_ADDED_ENCHANTMENTS.into(), Nbt::Compound(entries));
}

/// The enchantments the stack carries in full: the ones its modifiers give plus the ones a player
/// added. This is the set the game applied its own enchantment protection for.
pub(crate) fn carried_enchantments(nbt: &NbtCompound, env: &Environment) -> std::collections::BTreeMap<String, i32> {
    let mut levels = enchantments(nbt, env);
    for (name, level) in added_enchantments(nbt) { *levels.entry(name).or_insert(0) += level; }
    levels
}

/// The effective level of each severing module the tool's modifiers carry, in modifier order (SeveringModule, one per module).
pub(crate) fn severing_levels(nbt: &NbtCompound, env: &Environment) -> Vec<f32> {
    let saved = saved_modifiers(nbt, env.modifiers);
    saved.iter().flat_map(|(entry, modifier)| modifier.severing_levels(entry)).collect()
}

/// The looting level the tool's modifiers give its holder (LootingModifierHook, as the constant looting enchantment they publish).
pub(crate) fn looting(nbt: &NbtCompound, env: &Environment) -> i32 {
    enchantments(nbt, env).get("minecraft:looting").copied().unwrap_or(0).max(0)
}

/// ArmorLootingModifierHook over one worn piece: the looting its modifiers add to its wearer's kills while it sits in `slot`
/// (ArmorLootingModifierHook.getLooting runs every other worn piece's modules into the kill's looting total).
pub(crate) fn armor_looting(nbt: &NbtCompound, env: &Environment, slot: &str) -> i32 {
    with_tool_facts(nbt, env, |saved, facts| saved.iter().map(|(entry, modifier)| modifier.armor_looting(entry, facts, slot)).sum())
}

/// HarvestEnchantmentsModifierHook over one worn piece: the enchantments its modifiers lend the
/// stack its wearer breaks a block with, while the piece sits in `slot`.
pub(crate) fn armor_harvest_enchantments(nbt: &NbtCompound, env: &Environment, slot: &str) -> Vec<(String, i32)> {
    with_tool_facts(nbt, env, |saved, facts| saved.iter().flat_map(|(entry, modifier)| modifier.armor_harvest_enchantments(entry, facts, slot)).collect())
}

/// The tool actions the tool's modifiers perform on a right click, in modifier order.
pub(crate) fn tool_actions(nbt: &NbtCompound, env: &Environment) -> Vec<(crate::tool_actions::ToolAction, String, bool)> {
    with_tool_facts(nbt, env, |saved, facts| saved.iter().flat_map(|(entry, modifier)| modifier.tool_actions(entry, facts)).collect())
}

/// The walk transforms the tool's modifiers perform as their wearer walks, in modifier order: the modifier's id
/// (the wear cause), the action, the sound and the radius at the entry's effective level.
pub(crate) fn walk_transforms(nbt: &NbtCompound, env: &Environment) -> Vec<(Identifier, crate::tool_actions::ToolAction, String, f32)> {
    with_tool_facts(nbt, env, |saved, facts| saved.iter().flat_map(|(entry, modifier)| modifier.walk_transforms(entry, facts)).collect())
}

/// The boot walk modules over every modifier of the tool (ArmorWalkModifierHook), in modifier order.
pub(crate) fn walk_modules(nbt: &NbtCompound, env: &Environment) -> Vec<(Identifier, crate::modifier_modules::WalkModule)> {
    with_tool_facts(nbt, env, |saved, facts| saved.iter().flat_map(|(entry, modifier)| modifier.walk_modules(entry, facts)).collect())
}

/// The place glow modules over every modifier of the tool (PlaceGlowModule), in modifier order:
/// the modifier's id (the wear cause) and the wear each glow it places costs.
pub(crate) fn place_glow(nbt: &NbtCompound, env: &Environment) -> Vec<(Identifier, i32)> {
    with_tool_facts(nbt, env, |saved, _facts| saved.iter().flat_map(|(entry, modifier)| modifier.place_glow_modules(entry)).collect())
}

/// BlockItemProviderCapability over the tool: the block item its provider modules offer, the wear
/// consuming one costs and the providing modifier (CapabilityImpl runs the tool's entries in order
/// and takes the first non-empty answer). A broken tool provides nothing. Consuming is the
/// consumer's half: it wears the tool by the returned wear under the returned modifier
/// (BlockItemProviderModule.consumeBlockItem).
pub(crate) fn block_item_provider(nbt: &NbtCompound, env: &Environment) -> Option<(Identifier, Identifier, i32)> {
    if is_broken(nbt) { return None; }
    with_tool_facts(nbt, env, |saved, facts| saved.iter().find_map(|(entry, modifier)| modifier.block_item_provider(entry, facts).map(|(item, damage)| (entry.id.clone(), item, damage))))
}

/// ToolActionModifierHook over every modifier of the tool: whether one of them lets it perform the named tool action
/// (a tool actions module such as `item_hook`, `drill_attack`, `grapple_hook` or `shield_disable`).
pub(crate) fn can_perform_action(nbt: &NbtCompound, env: &Environment, name: &str) -> bool {
    with_tool_facts(nbt, env, |saved, facts| saved.iter().any(|(entry, modifier)| modifier.can_perform_action(entry, facts, name)))
}

/// What the tool's modifiers add to the protection cap attribute (boundless).
pub(crate) fn protection_cap(nbt: &NbtCompound, env: &Environment) -> f32 {
    with_tool_facts(nbt, env, |saved, facts| saved.iter().map(|(entry, modifier)| modifier.protection_cap(entry, facts, nbt)).sum())
}

/// ProtectionModifierHook over every modifier of the tool, in modifier order, starting from `value`.
pub(crate) fn protection(nbt: &NbtCompound, env: &Environment, source: &crate::protection::DamageSource, value: f32) -> f32 {
    with_tool_facts(nbt, env, |saved, facts| saved.iter().fold(value, |value, (entry, modifier)| modifier.protection(entry, facts, nbt, source, value)))
}

/// ProtectionModifierHook as `protection` runs it, and the edits of the piece's persistent data that what the modules spent (the overslime an
/// overshield converts) leaves behind. Each modifier's modules see the overslime the earlier ones left.
pub(crate) fn protection_with_edits(nbt: &NbtCompound, env: &Environment, source: &crate::protection::DamageSource, value: f32) -> (f32, Vec<pluto_lib::StackDataEdit>) {
    with_tool_facts(nbt, env, |saved, facts| {
        let mut work = nbt.clone();
        let value = saved.iter().fold(value, |value, (entry, modifier)| {
            let value = modifier.protection(entry, facts, &work, source, value);
            modifier.modules().iter().fold(value, |value, module| module.overshield(entry, &mut work, source, value))
        });
        (value, crate::capacity::persistent_edits(nbt, &work))
    })
}

/// What the restore-lost-health modules of a worn piece do for `amount` of damage its wearer took at the stage `hurt`
/// names, with the modifier each belongs to (the cause of the wear it costs), in modifier order. `roll` gives uniform
/// values in [0, 1).
pub(crate) fn restores(nbt: &NbtCompound, env: &Environment, source: &crate::protection::DamageSource, amount: f32, hurt: bool, roll: &mut dyn FnMut() -> f32) -> Vec<(Identifier, crate::wearer_modules::Restore)> {
    with_tool_facts(nbt, env, |saved, facts| {
        let mut found = Vec::new();
        for (entry, modifier) in saved {
            for module in modifier.modules() {
                if let Some(restore) = module.restore_lost_health(entry, facts, source, amount, hurt, roll) { found.push((entry.id.clone(), restore)); }
            }
        }
        found
    })
}

/// What the teleport-dodge modules of a worn piece do for `amount` of damage its wearer took: the dodge of each module
/// whose conditions and chance land, with the modifier it belongs to (the cause of the wear it costs), in modifier
/// order. `enderference` says whether the wearer already has the effect, which locks the dodge out. The caller
/// performs the teleport the dodge is named for and prevents the damage. `roll` gives uniform values in [0, 1).
pub(crate) fn dodges(nbt: &NbtCompound, env: &Environment, source: &crate::protection::DamageSource, enderference: bool, amount: f32, roll: &mut dyn FnMut() -> f32) -> Vec<(Identifier, crate::teleport_dodge::Dodge)> {
    with_tool_facts(nbt, env, |saved, facts| {
        let mut found = Vec::new();
        for (entry, modifier) in saved {
            for module in modifier.modules() {
                if let Some(dodge) = module.teleport_dodge(entry, facts, source, enderference, amount, roll) { found.push((entry.id.clone(), dodge)); }
            }
        }
        found
    })
}

/// What taking a worn piece off does to its wearer besides clearing effects: the hurts (damage type, amount) its damage-on-unequip modules deal
/// and the shares (category, percent) its reduce-effect modules take off the wearer's effects, in modifier order.
pub(crate) fn unequip_effects(nbt: &NbtCompound, env: &Environment) -> (Vec<(String, f32)>, Vec<(pluto_lib::EffectCategory, f32)>) {
    with_tool_facts(nbt, env, |saved, facts| {
        let (mut hurts, mut reductions) = (Vec::new(), Vec::new());
        for (entry, modifier) in saved {
            for module in modifier.modules() {
                hurts.extend(module.damage_on_unequip(entry, facts));
                reductions.extend(module.reduce_effect_on_unequip(entry, facts));
            }
        }
        (hurts, reductions)
    })
}

/// ModifyDamageModifierHook over every modifier of the tool, in modifier order, starting from `amount`: the damage its wearer takes.
pub(crate) fn adjust_damage(nbt: &NbtCompound, env: &Environment, source: &crate::protection::DamageSource, entities: &crate::entity_facts::Entities, amount: f32) -> f32 {
    with_tool_facts(nbt, env, |saved, facts| saved.iter().fold(amount, |amount, (entry, modifier)| if amount > 0.0 { modifier.adjust_damage(entry, facts, nbt, source, entities, amount) } else { amount }))
}

/// The attribute modifiers the tool's modifiers add for the `wanted` slots (AttributesModifierHook), in modifier order.
pub(crate) fn attribute_entries(nbt: &NbtCompound, env: &Environment, wanted: &[&str]) -> Vec<NbtCompound> {
    with_tool_facts(nbt, env, |saved, facts| saved.iter().flat_map(|(entry, modifier)| modifier.attribute_entries(entry, facts, nbt, wanted)).collect())
}

/// The contributions the tool's modifiers make to declared attributes for the stack's `slot`, as
/// (attribute, operation, amount) in modifier order: the custom attribute modules only, whose amounts
/// are worked out by the port (custom_attributes) rather than published on the stack.
pub(crate) fn custom_attribute_amounts(nbt: &NbtCompound, env: &Environment, slot: &str) -> Vec<(String, pluto_lib::AttributeOperation, f64)> {
    with_tool_facts(nbt, env, |saved, facts| saved.iter().flat_map(|(entry, modifier)| modifier.modules().iter().filter_map(|module| module.custom_attribute_amount(entry, facts, nbt, slot))).collect())
}

/// MeleeDamageModifierHook over every modifier of the tool: each conditional melee damage module in
/// priority order (highest first, ties in modifier order), starting from `damage`. `entities` are the attacker and target as PlutoLib
/// reported them, for the modules that read their health, temperature or effects.
pub(crate) fn melee_damage(nbt: &NbtCompound, env: &Environment, base: f32, damage: f32, target_in_tag: &dyn Fn(&Identifier) -> bool, entities: &crate::entity_facts::Entities) -> f32 {
    with_tool_facts(nbt, env, |saved, facts| {
        let mut modules: Vec<(i32, usize, &crate::modifier_entries::Entry, &crate::modifier_modules::Module)> = Vec::new();
        for (index, (entry, modifier)) in saved.iter().enumerate() {
            for (priority, module) in modifier.melee_modules() { modules.push((priority, index, entry, module)); }
        }
        modules.sort_by_key(|(priority, index, _, _)| (std::cmp::Reverse(*priority), *index));
        let view = crate::mining_speed::ToolView::with(nbt, *entities);
        modules.into_iter().fold(damage, |damage, (_, _, entry, module)| module.melee_damage(entry, facts, &view, base, damage, target_in_tag))
    })
}

/// ConditionalStatModifierHook.getModifiedStat: the tool's stored `stat` after each modifier's conditional stat modules (a modifier's
/// modules by priority, the modifiers in tool order), clamped to the stat's limits. `holder_in_tag` answers the holder's pseudo state tags
/// and `entities.attacker` is the holder's state, for the variables that read it. A stat the item does not support is returned as stored.
pub(crate) fn conditional_stat(nbt: &NbtCompound, env: &Environment, stat: Stat, holder_in_tag: &dyn Fn(&Identifier) -> bool, entities: &crate::entity_facts::Entities) -> f32 {
    with_tool_facts(nbt, env, |saved, facts| {
        let mut value = self::stat(nbt, stat);
        if !crate::modifier_modules::supports(stat, facts) { return stat.clamp(value); }
        let view = crate::mining_speed::ToolView::with(nbt, *entities);
        let multiplier = view.multiplier(stat);
        for (entry, modifier) in saved {
            let mut modules: Vec<(i32, &crate::modifier_modules::Module)> = modifier.conditional_stat_modules().collect();
            modules.sort_by_key(|(priority, _)| std::cmp::Reverse(*priority));
            for (_, module) in modules { value = module.conditional_stat(entry, facts, &view, holder_in_tag, stat, value, multiplier); }
        }
        stat.clamp(value)
    })
}

/// ProjectileShootModifierHook for every conditional power module of the tool: its projectile damage multiplier is recorded in the
/// projectile's persistent data (separately for ammo and for a launcher).
pub(crate) fn power_shoot(nbt: &NbtCompound, env: &Environment, is_ammo: bool, projectile: &mut NbtCompound) {
    with_tool_facts(nbt, env, |saved, _| {
        let multiplier = crate::mining_speed::ToolView::of(nbt).multiplier(Stat::ProjectileDamage);
        for (_, modifier) in saved { for (_, module) in modifier.conditional_power_modules() { module.shoot(is_ammo, multiplier, projectile); } }
    })
}

/// ProjectileHitModifierHook for every conditional power module of the shooter's tool, in priority order (highest first, ties in
/// modifier order): the arrow's base damage after them, each modifier boosting once per projectile.
pub(crate) fn power_hit(nbt: &NbtCompound, env: &Environment, projectile: &mut NbtCompound, context: &crate::conditional_stat::PowerContext, base_damage: f32) -> f32 {
    with_tool_facts(nbt, env, |saved, _| {
        let mut modules: Vec<(i32, usize, &crate::modifier_entries::Entry, &crate::conditional_stat::ConditionalPower)> = Vec::new();
        for (index, (entry, modifier)) in saved.iter().enumerate() {
            for (priority, module) in modifier.conditional_power_modules() { modules.push((priority, index, entry, module)); }
        }
        modules.sort_by_key(|(priority, index, _, _)| (std::cmp::Reverse(*priority), *index));
        modules.into_iter().fold(base_damage, |damage, (_, _, entry, module)| module.hit(entry, projectile, context, damage))
    })
}

/// ProjectileShootModifierHook for every punch module of the tool, in priority order (highest first, ties in modifier
/// order): the knockback the shot leaves with, from a base of 0. On a plain arrow a module sets the knockback, so the
/// last to run is what stands; on a projectile with knockback of its own each module adds its amount.
pub(crate) fn punch_shoot(nbt: &NbtCompound, env: &Environment, arrow: bool) -> f32 {
    with_tool_facts(nbt, env, |saved, facts| {
        let mut modules: Vec<(i32, usize, crate::punch::Shot)> = Vec::new();
        for (index, (entry, modifier)) in saved.iter().enumerate() {
            for module in modifier.modules() {
                if let Some(shot) = module.punch(entry, facts, arrow) { modules.push((modifier.priority, index, shot)); }
            }
        }
        modules.sort_by_key(|(priority, index, _)| (std::cmp::Reverse(*priority), *index));
        modules.into_iter().fold(0.0, |knockback, (_, _, shot)| match shot {
            crate::punch::Shot::Set(value) => value as f32,
            crate::punch::Shot::Add(value) => knockback + value,
        })
    })
}

/// ProjectileHitModifierHook for every reverse punch module of the tool: the push the entity the projectile hit takes,
/// the modules' pushes added up, or None when none of them pushes. `owned` says the projectile has an owner or is an
/// arrow, `resistance` is the target's knockback resistance and `motion` the projectile's horizontal motion (x, z).
pub(crate) fn reverse_punch_push(nbt: &NbtCompound, env: &Environment, owned: bool, resistance: f32, motion: [f64; 2]) -> Option<[f64; 3]> {
    with_tool_facts(nbt, env, |saved, _| {
        let mut push: Option<[f64; 3]> = None;
        for (entry, modifier) in saved {
            for module in modifier.modules() {
                if let Some(module) = module.reverse_punch() {
                    if let Some(one) = module.push(entry, owned, resistance, motion) {
                        let total = push.get_or_insert([0.0; 3]);
                        total[0] += one[0]; total[1] += one[1]; total[2] += one[2];
                    }
                }
            }
        }
        push
    })
}

/// MeleeHitModifierHook.beforeMeleeHit's knockback over every modifier of the tool, in priority order (highest first,
/// ties in modifier order), starting from the game's `strength`.
pub(crate) fn melee_knockback(nbt: &NbtCompound, env: &Environment, strength: f32, target_in_tag: &dyn Fn(&Identifier) -> bool) -> f32 {
    with_tool_facts(nbt, env, |saved, facts| {
        let mut modules: Vec<(i32, usize, &crate::modifier_entries::Entry, &crate::modifier_modules::Module)> = Vec::new();
        for (index, (entry, modifier)) in saved.iter().enumerate() {
            for (priority, module) in modifier.knockback_modules() { modules.push((priority, index, entry, module)); }
        }
        modules.sort_by_key(|(priority, index, _, _)| (std::cmp::Reverse(*priority), *index));
        let view = crate::mining_speed::ToolView::of(nbt);
        modules.into_iter().fold(strength, |strength, (_, _, entry, module)| module.knockback(entry, facts, &view, strength, target_in_tag))
    })
}

/// The mob effects a melee hit with this tool puts on its target, as (applies before the hit, grant), in modifier order. `in_tag` answers
/// the entity type tags and the pseudo state tags of the hit; `rolls` gives uniform values in [0, 1).
pub(crate) fn weapon_effects(nbt: &NbtCompound, env: &Environment, in_tag: &dyn Fn(&Identifier) -> bool, rolls: &mut dyn FnMut() -> f32) -> Vec<(bool, crate::weapon_effect::Grant)> {
    with_tool_facts(nbt, env, |saved, facts| {
        let mut grants = Vec::new();
        for (entry, modifier) in saved {
            for module in modifier.weapon_effect_modules() {
                if let Some(grant) = module.weapon_effect(entry, facts, in_tag, rolls) { grants.push(grant); }
            }
        }
        grants
    })
}

/// The mob effects this tool puts on its own holder for a use of it (a swing, a shot, shearing a mob), in modifier order. `rolls` gives uniform
/// values in [0, 1).
pub(crate) fn usage_effects(nbt: &NbtCompound, env: &Environment, usage: crate::weapon_effect::Usage, rolls: &mut dyn FnMut() -> f32) -> Vec<crate::weapon_effect::Grant> {
    with_tool_facts(nbt, env, |saved, facts| {
        let mut grants = Vec::new();
        for (entry, modifier) in saved {
            for module in modifier.usage_effect_modules() {
                if let Some(grant) = module.usage_effect(entry, facts, usage, rolls) { grants.push(grant); }
            }
        }
        grants
    })
}

/// Whether a worn armor piece blocks damage from `source` altogether (DamageBlockModifierHook).
pub(crate) fn blocks_damage(nbt: &NbtCompound, env: &Environment, source: &crate::protection::DamageSource) -> bool {
    with_tool_facts(nbt, env, |saved, facts| {
        saved.iter().any(|(entry, modifier)| modifier.block_damage_modules().any(|module| module.blocks_damage(entry, facts, source)))
    })
}

/// The effects a worn armor piece makes its wearer immune to, each with the count the piece adds, in modifier order.
pub(crate) fn effect_immunities(nbt: &NbtCompound, env: &Environment) -> Vec<(Identifier, i32)> {
    with_tool_facts(nbt, env, |saved, facts| {
        let mut counts = Vec::new();
        for (entry, modifier) in saved {
            for module in modifier.effect_immunity_modules() {
                if let Some(count) = module.effect_immunity(entry, facts) { counts.push(count); }
            }
        }
        counts
    })
}

/// The effects taking a worn armor piece off takes off its wearer (clear-effect-on-unequip modules), in modifier order.
pub(crate) fn effects_cleared_on_unequip(nbt: &NbtCompound, env: &Environment) -> Vec<Identifier> {
    with_tool_facts(nbt, env, |saved, facts| saved.iter().flat_map(|(entry, modifier)| modifier.modules().iter().filter_map(|module| module.clear_effect_on_unequip(entry, facts))).collect())
}

/// The claims a worn armor piece makes on attributes through max armor attribute modules, in modifier order.
pub(crate) fn max_armor_claims(nbt: &NbtCompound, env: &Environment) -> Vec<crate::max_armor::Candidate> {
    let broken = is_broken(nbt);
    with_tool_facts(nbt, env, |saved, facts| {
        saved.iter().flat_map(|(entry, modifier)| modifier.max_armor_attribute_modules().filter_map(move |module| module.max_armor_candidate(entry, facts, broken)).collect::<Vec<_>>()).collect()
    })
}

/// Whether a worn armor piece carries a golden attribute module.
pub(crate) fn has_golden_modules(nbt: &NbtCompound, env: &Environment) -> bool {
    saved_modifiers(nbt, env.modifiers).iter().any(|(_, modifier)| modifier.golden_attribute_modules().next().is_some())
}

/// The (modifier, attribute, operation, amount) claims a worn armor piece makes through golden attribute modules for a
/// wearer with `total_gold` golden pieces, in modifier order.
pub(crate) fn golden_amounts(nbt: &NbtCompound, env: &Environment, total_gold: i32) -> Vec<(Identifier, String, pluto_lib::AttributeOperation, f64)> {
    with_tool_facts(nbt, env, |saved, facts| {
        saved.iter().flat_map(|(entry, modifier)| modifier.golden_attribute_modules().filter_map(move |module| module.golden_amount(entry, facts, total_gold).map(|(attribute, operation, amount)| (entry.id.clone(), attribute, operation, amount))).collect::<Vec<_>>()).collect()
    })
}

/// The lightspeed modules a worn piece carries, each with its modifier's id and the entry's effective level,
/// in modifier order.
pub(crate) fn lightspeed(nbt: &NbtCompound, env: &Environment) -> Vec<(Identifier, crate::wearer_modules::LightspeedAttribute, f32)> {
    with_tool_facts(nbt, env, |saved, facts| {
        saved.iter().flat_map(|(entry, modifier)| modifier.lightspeed_attribute_modules().filter_map(move |module| module.lightspeed(entry, facts)).map(|(module, level)| (entry.id.clone(), module.clone(), level)).collect::<Vec<_>>()).collect()
    })
}

/// The levels a worn armor piece adds under each armor level key, in modifier order; `slot` is the slot the piece is worn in.
pub(crate) fn armor_levels(nbt: &NbtCompound, env: &Environment, slot: &str) -> Vec<(Identifier, i32)> {
    let broken = is_broken(nbt);
    saved_modifiers(nbt, env.modifiers).iter().flat_map(|(entry, modifier)| modifier.armor_level_modules().filter_map(move |module| module.armor_level(entry, slot, broken)).collect::<Vec<_>>()).collect()
}
/// EdibleModule: whether the tool carries the edible module, and the effects eating it has, in modifier order.
pub(crate) fn edible(nbt: &NbtCompound, env: &Environment) -> Option<Vec<crate::edible::Effect>> {
    with_tool_facts(nbt, env, |saved, facts| {
        if !saved.iter().any(|(_, modifier)| modifier.is_edible()) { return None; }
        Some(saved.iter().flat_map(|(entry, modifier)| modifier.edible_effects(entry, facts)).collect())
    })
}

/// The items the tool's modifiers present as eaten (EdibleRepresentativeItemModule.onToolEaten's
/// list, in modifier order): what the eating presentation sounds like, and what an integration
/// would be told was eaten.
pub(crate) fn representative_items(nbt: &NbtCompound, env: &Environment) -> Vec<Identifier> {
    with_tool_facts(nbt, env, |saved, facts| saved.iter().flat_map(|(entry, modifier)| modifier.representative_items(entry, facts)).collect())
}
/// The mob types a worn armor piece makes its wearer pass for, each with the count the piece adds (its modifier's level), in modifier order.
pub(crate) fn mob_disguises(nbt: &NbtCompound, env: &Environment) -> Vec<(Identifier, i32)> {
    let mut disguises = Vec::new();
    for (entry, modifier) in saved_modifiers(nbt, env.modifiers) {
        for module in modifier.mob_disguise_modules() {
            if let Some(disguise) = module.mob_disguise(&entry) { disguises.push(disguise); }
        }
    }
    disguises
}

/// ModifierUtil.checkVolatileFlag: whether the tool's volatile data holds the flag.
pub(crate) fn volatile_flag(nbt: &NbtCompound, flag: &str) -> bool {
    matches!(nbt.get(TAG_VOLATILE), Some(Nbt::Compound(data)) if boolean(data, flag))
}

/// The damage a worn armor piece's thorns deal to the mob that hit its wearer, in modifier order. `in_tag` answers the entity type tags and
/// the pseudo state tags of the attacking mob; `rolls` gives uniform values in [0, 1).
pub(crate) fn thorns_hurts(nbt: &NbtCompound, env: &Environment, in_tag: &dyn Fn(&Identifier) -> bool, rolls: &mut dyn FnMut() -> f32) -> Vec<crate::thorns::Hurt> {
    with_tool_facts(nbt, env, |saved, facts| {
        let mut hurts = Vec::new();
        for (entry, modifier) in saved {
            for module in modifier.thorns_modules() {
                if let Some(hurt) = module.thorns(entry, facts, in_tag, rolls) { hurts.push(hurt); }
            }
        }
        hurts
    })
}

/// The actions a melee hit with this tool does to the mob it hit (fire, freezing), in modifier order.
pub(crate) fn attack_actions(nbt: &NbtCompound, env: &Environment) -> Vec<pluto_lib::EntityAction> {
    with_tool_facts(nbt, env, |saved, facts| {
        saved.iter().flat_map(|(entry, modifier)| modifier.attack_action_modules().filter_map(move |module| module.attack_action(entry, facts)).collect::<Vec<_>>()).collect()
    })
}

/// The edits of the persistent data a melee hit with this tool makes through its melee capacity modules (each module in modifier order, on
/// the result of the one before). `target_in_tag` and `attacker_in_tag` answer the entity type and state tags of the mob hit and of the player.
pub(crate) fn melee_capacity_edits(nbt: &NbtCompound, env: &Environment, target_in_tag: &dyn Fn(&Identifier) -> bool, attacker_in_tag: &dyn Fn(&Identifier) -> bool) -> Vec<pluto_lib::StackDataEdit> {
    with_tool_facts(nbt, env, |saved, facts| {
        let mut work = nbt.clone();
        for (entry, modifier) in saved {
            for module in modifier.melee_capacity_modules() {
                let Some((owner, grant)) = module.melee_capacity(entry, facts, target_in_tag, attacker_in_tag) else { continue };
                if let Some(target) = crate::capacity::Target::find(saved, entry, owner) { target.grant(&mut work, env, grant); }
            }
        }
        crate::capacity::persistent_edits(nbt, &work)
    })
}

/// The edits of the persistent data one block broken with this tool makes through its mining capacity modules (each in modifier order, on
/// the result of the one before): the modules that run before the break (`before`) or after it.
pub(crate) fn mining_capacity_edits(nbt: &NbtCompound, env: &Environment, before: bool) -> Vec<pluto_lib::StackDataEdit> {
    with_tool_facts(nbt, env, |saved, facts| {
        let mut work = nbt.clone();
        for (entry, modifier) in saved {
            for module in modifier.mining_capacity_modules() {
                let Some((owner, grant)) = module.mining_capacity(entry, facts, before) else { continue };
                if let Some(target) = crate::capacity::Target::find(saved, entry, owner) { target.grant(&mut work, env, grant); }
            }
        }
        crate::capacity::persistent_edits(nbt, &work)
    })
}

/// The edits of the persistent data one projectile launched with this tool makes through its launch capacity modules (each in modifier order,
/// on the result of the one before).
pub(crate) fn launch_capacity_edits(nbt: &NbtCompound, env: &Environment) -> Vec<pluto_lib::StackDataEdit> {
    with_tool_facts(nbt, env, |saved, facts| {
        let mut work = nbt.clone();
        for (entry, modifier) in saved {
            for module in modifier.launch_capacity_modules() {
                let Some((owner, grant)) = module.launch_capacity(entry, facts) else { continue };
                if let Some(target) = crate::capacity::Target::find(saved, entry, owner) { target.grant(&mut work, env, grant); }
            }
        }
        crate::capacity::persistent_edits(nbt, &work)
    })
}

/// MeltingModule.processLoot for block loot: drops the tool can melt become its tank's fluid, under
/// the module's temperature and its own ore rates (tool_tank::Melting). `force` is the caller-computed
/// force-melting state (the volatile flag is set and the tool is effective on the block); `melt`
/// resolves one dropped stack to (fluid, temperature) for a module and entry level, or None when the
/// stack has no melting recipe. The original skips a full tank entirely; unforced melting takes only
/// the copies that fit, forced melting takes them all and deletes what cannot melt. Returns the
/// tool's persistent-data edits.
pub(crate) fn melting_loot(
    nbt: &mut NbtCompound,
    env: &Environment,
    loot: &mut Vec<pluto_lib::ItemSlot>,
    force: bool,
    melt: &dyn Fn(&pluto_lib::ItemSlot, &crate::tool_tank::Melting, f32) -> Option<(pluto_lib::FluidStack, i32)>,
) -> Vec<pluto_lib::StackDataEdit> {
    let mut work = nbt.clone();
    with_tool_facts(nbt, env, |saved, facts| {
        for (entry, modifier) in saved {
            for module in modifier.modules() {
                let Some(module) = module.melting(entry, facts) else { continue; };
                if crate::tool_tank::capacity(&work) <= 0 { continue; }
                let mut index = 0;
                while index < loot.len() {
                    // The original checks the tank after every drop: a full tank ends the pass.
                    let current = crate::tool_tank::fluid(&work);
                    if current.as_ref().is_some_and(|stored| stored.amount as i32 >= crate::tool_tank::capacity(&work)) { break; }
                    let Some((fluid, temperature)) = melt(&loot[index], module, entry.effective_level()) else {
                        if force { loot.remove(index); } else { index += 1; }
                        continue;
                    };
                    if temperature > module.temperature(entry.effective_level()) || fluid.amount == 0 {
                        if force { loot.remove(index); } else { index += 1; }
                        continue;
                    }
                    if current.as_ref().is_some_and(|stored| !stored.same_fluid(&fluid)) {
                        index += 1;
                        continue;
                    }
                    let per_copy = fluid.amount as i32;
                    let room = crate::tool_tank::capacity(&work) - current.as_ref().map_or(0, |stored| stored.amount as i32);
                    let fitting = if force { loot[index].count as i32 } else { (room / per_copy).min(loot[index].count as i32) };
                    if fitting > 0 {
                        let mut fill_stack = fluid.clone();
                        fill_stack.amount = (per_copy * fitting) as u32;
                        crate::tool_tank::fill(&mut work, &fill_stack, true);
                        if fitting >= loot[index].count as i32 { loot.remove(index); continue; }
                        loot[index].count -= fitting as u32;
                    }
                    index += 1;
                }
            }
        }
    });
    let edits = crate::capacity::persistent_edits(nbt, &work);
    *nbt = work;
    edits
}

/// InventoryTickModifierHook for every overburn module of the tool: burns the tank's fluid as
/// melting fuel to regrow overslime (tool_tank::overburn_tick). `fuel` resolves a fluid to its
/// melting fuel facts; `game_time` is the world clock the burn's expiration is measured on. Whether
/// the tool changed.
pub(crate) fn overburn_tick(
    nbt: &mut NbtCompound,
    env: &Environment,
    holder_tick: i32,
    game_time: i64,
    using_this: bool,
    fuel: &dyn Fn(&Identifier) -> Option<crate::tool_tank::Fuel>,
    rolls: &mut dyn FnMut() -> f32,
) -> bool {
    let snapshot = nbt.clone();
    let mut changed = false;
    with_tool_facts(&snapshot, env, |saved, _facts| {
        for (entry, modifier) in saved {
            if modifier.modules().iter().any(|module| module.is_overburn()) {
                changed |= crate::tool_tank::overburn_tick(nbt, env, entry, holder_tick, game_time, using_this, fuel, rolls);
            }
        }
    });
    changed
}

/// The health a critical melee hit with this tool that dealt `damage` gives its wielder through the tool's lifesteal modules (added up, in
/// modifier order). `blacklisted` says the mob hit is of the necrotic blacklist; the tags answer the entity type and state tags of the player
/// and of the mob.
pub(crate) fn lifesteal_heal(nbt: &NbtCompound, env: &Environment, damage: f32, blacklisted: bool, attacker_in_tag: &dyn Fn(&Identifier) -> bool, target_in_tag: &dyn Fn(&Identifier) -> bool) -> f32 {
    with_tool_facts(nbt, env, |saved, facts| {
        saved.iter().flat_map(|(entry, modifier)| modifier.lifesteal_modules().filter_map(move |module| module.lifesteal(entry, facts, damage, blacklisted, attacker_in_tag, target_in_tag)).collect::<Vec<_>>()).sum()
    })
}

/// MeleeHitModifierHook for every melee instrument module of the tool: the sound each plays for the hit, in modifier
/// order. The closures answer the entity type tags and the pseudo state tags of the attacker and of the target; `roll`
/// picks within an instrument tag, as the original draws from the tag.
pub(crate) fn melee_instrument_sound(nbt: &NbtCompound, env: &Environment, attacker_in_tag: &dyn Fn(&Identifier) -> bool, target_in_tag: &dyn Fn(&Identifier) -> bool, roll: f32) -> Vec<crate::melee_instrument::Sound> {
    with_tool_facts(nbt, env, |saved, facts| {
        let list = materials(nbt, env.has_materials);
        let mut sounds = Vec::new();
        for (entry, modifier) in saved {
            for module in modifier.modules() {
                if let Some(sound) = module.melee_instrument(entry, facts, list.as_slice(), attacker_in_tag, target_in_tag, roll) { sounds.push(sound); }
            }
        }
        sounds
    })
}

/// InventoryTickModifierHook for every overgrowth module of the tool: whether the tool grew overslime on this tick of
/// its holder. The original runs the hook once a second (the holder's tick count a multiple of 20), on the server, and
/// skips a stack the holder is using; `rolls` gives one uniform value in [0, 1) per overgrowth module.
pub(crate) fn overgrowth_tick(nbt: &mut NbtCompound, env: &Environment, holder_tick: i32, using_this: bool, rolls: &mut dyn FnMut() -> f32) -> bool {
    if holder_tick % 20 != 0 || using_this { return false; }
    let snapshot = nbt.clone();
    with_tool_facts(&snapshot, env, |saved, facts| {
        let mut grew = false;
        for (entry, modifier) in saved {
            for module in modifier.modules() {
                if module.is_overgrowth() && module.overgrowth(entry, facts, &mut *nbt, env, rolls()) { grew = true; }
            }
        }
        grew
    })
}

/// ModifierUtil.getModifierLevel: the tool's level of the named modifier, 0 when it does not carry it.
pub(crate) fn modifier_level(nbt: &NbtCompound, env: &Environment, name: &str) -> i32 {
    saved_modifiers(nbt, env.modifiers).iter().find(|(entry, _)| entry.id.to_string() == name).map_or(0, |(entry, _)| entry.level)
}

/// The total kinetic level of a piece: the plain levels of the entries whose modifier carries a kinetic module, as
/// SlotInChargeModule totals the equipped pieces. A broken piece's modifiers do not run, so it contributes nothing.
pub(crate) fn kinetic_level(nbt: &NbtCompound, env: &Environment) -> i32 {
    if is_broken(nbt) { return 0; }
    saved_modifiers(nbt, env.modifiers).iter().flat_map(|(entry, modifier)| modifier.modules().iter().filter_map(|module| module.kinetic_level(entry))).sum()
}

/// InsatiableModifier.afterMeleeHit: the grant a melee hit of this tool feeds its holder's
/// insatiable (melee) effect — one level more than the holder carries, up to the fifth, for
/// 100 ticks; None when the tool has no insatiable module or is broken. The swing's charge is
/// not reported, so every hit counts as fully charged, as the lifesteal module reads a swing.
pub(crate) fn insatiable_hit_grant(nbt: &NbtCompound, env: &Environment, attacker: Option<&pluto_lib::EntityState>) -> Option<crate::weapon_effect::Grant> {
    if is_broken(nbt) { return None; }
    let carried = saved_modifiers(nbt, env.modifiers).iter()
        .any(|(_, modifier)| modifier.modules().iter().any(|module| module.is_insatiable()));
    if !carried { return None; }
    let level = crate::entity_facts::effect_level_of(attacker, "tconstruct:insatiable_melee");
    // applyEffect: the new amplifier is the current one plus one, capped at 4 (the fifth level).
    let amplifier = (level as i32).min(4);
    Some(crate::weapon_effect::Grant { effect: Identifier::new("tconstruct", "insatiable_melee").expect("valid effect id"), amplifier, duration: 100 })
}

/// The bleeding grant LaceratingModifier.applyEffect builds: the amplifier is the hit's looting,
/// the duration the modifier's level in seconds (level * 2 * 20 ticks) plus the 19 ticks the
/// original adds when the effect is not already running at that amplifier. A running bleeding's
/// remaining ticks are not reported, so the same-amplifier extension (which adds those ticks
/// instead) cannot be worked out; the 19-tick branch always runs.
fn bleeding_grant(level: i32, looting: i32) -> crate::weapon_effect::Grant {
    crate::weapon_effect::Grant { effect: Identifier::new("tconstruct", "bleeding").expect("valid effect id"), amplifier: looting.max(0), duration: level * 40 + 19 }
}

/// LaceratingModifier.afterMeleeHit: the bleeding grant for the mob a melee hit of this tool
/// landed on; its looting is the weapon's own looting plus the looting of the armor the attacker
/// wears (LootingModifierHook and ArmorLootingModifierHook over the hit). None when the tool has
/// no lacerating module or is broken. The swing's charge is not reported, so every hit counts as
/// fully charged, as the lifesteal module reads a swing.
pub(crate) fn lacerating_hit_grant(nbt: &NbtCompound, env: &Environment, armor: &[(String, pluto_lib::StackData)]) -> Option<crate::weapon_effect::Grant> {
    if is_broken(nbt) { return None; }
    let level = saved_modifiers(nbt, env.modifiers).iter()
        .flat_map(|(entry, modifier)| modifier.modules().iter().filter_map(|module| module.lacerating_level(entry))).next()?;
    let mut total = looting(nbt, env);
    for (slot, piece) in armor { total += armor_looting(&piece.custom_data, env, slot); }
    Some(bleeding_grant(level, total))
}

/// LaceratingModifier.onAttacked: the bleeding grant a worn piece may put on the mob that hit
/// its wearer, when the quarter chance lands (doubled on a blocking wearer in the original; the
/// wearer's blocking is not reported by the hit, so the base chance always runs). Its looting is
/// the wearer's armor looting over all the worn pieces, this piece's own included. None when the
/// piece has no lacerating module, is broken, or the chance misses.
pub(crate) fn lacerating_counter_grant(nbt: &NbtCompound, env: &Environment, armor: &[(String, pluto_lib::StackData)], rolls: &mut dyn FnMut() -> f32) -> Option<crate::weapon_effect::Grant> {
    if is_broken(nbt) { return None; }
    let level = saved_modifiers(nbt, env.modifiers).iter()
        .flat_map(|(entry, modifier)| modifier.modules().iter().filter_map(|module| module.lacerating_level(entry))).next()?;
    if rolls() >= 0.25 { return None; }
    let mut total = looting(nbt, env);
    for (slot, piece) in armor { total += armor_looting(&piece.custom_data, env, slot); }
    Some(bleeding_grant(level, total))
}

/// EnderclearanceModule: the (chance, diameter) pairs of this stack's enderclearance modules, in
/// modifier order; empty when it has none or is broken.
pub(crate) fn enderclearance(nbt: &NbtCompound, env: &Environment) -> Vec<(f32, f32)> {
    if is_broken(nbt) { return Vec::new(); }
    saved_modifiers(nbt, env.modifiers).iter()
        .flat_map(|(entry, modifier)| modifier.modules().iter().filter_map(|module| module.enderclearance(entry)))
        .collect()
}

/// ChannelingModule (melee form): the (clear, rain, thunder) chances of this stack's channeling
/// modules that allow melee; empty when it has none, they forbid melee, or it is broken.
pub(crate) fn channeling_melee(nbt: &NbtCompound, env: &Environment) -> Vec<(f32, f32, f32)> {
    if is_broken(nbt) { return Vec::new(); }
    saved_modifiers(nbt, env.modifiers).iter()
        .flat_map(|(_, modifier)| modifier.modules().iter().filter_map(|module| module.channeling_melee()))
        .collect()
}

/// The recurrent protection modules of a piece: the plain level of each entry whose modifier carries one, with the
/// module, in modifier order, as SlotInChargeModule totals the equipped pieces. A broken piece's modifiers do not
/// run, so it contributes nothing.
pub(crate) fn recurrent_protection(nbt: &NbtCompound, env: &Environment) -> Vec<(i32, crate::wearer_modules::RecurrentProtection)> {
    if is_broken(nbt) { return Vec::new(); }
    saved_modifiers(nbt, env.modifiers).iter().flat_map(|(entry, modifier)| modifier.modules().iter().filter_map(|module| module.recurrent_protection(entry))).collect()
}

/// Whether any of the tool's modifiers lets it block (BlockingModule), so a right click raises it like a shield.
/// A broken piece's modifiers do not run, so it contributes nothing.
pub(crate) fn blocking(nbt: &NbtCompound, env: &Environment) -> bool {
    if is_broken(nbt) { return false; }
    saved_modifiers(nbt, env.modifiers).iter().any(|(_, modifier)| modifier.modules().iter().any(|module| module.blocking()))
}

/// Whether any of the tool's modifiers puts out a lit campfire the tool is used on (ExtinguishCampfireModule).
pub(crate) fn extinguishes_campfires(nbt: &NbtCompound, env: &Environment) -> bool {
    with_tool_facts(nbt, env, |saved, facts| saved.iter().any(|(entry, modifier)| modifier.modules().iter().any(|module| module.extinguishes_campfires(entry, facts))))
}

/// What the tool's damage on shoot modules hurt its holder by for firing it, in modifier order.
pub(crate) fn damage_on_shoot(nbt: &NbtCompound, env: &Environment) -> Vec<(String, f32)> {
    with_tool_facts(nbt, env, |saved, facts| saved.iter().flat_map(|(entry, modifier)| modifier.modules().iter().filter_map(|module| module.damage_on_shoot(entry, facts))).collect())
}

/// The tool's sling modules in modifier order, each with its entry and its modifier's ID:
/// the release computation runs them in order, as the original's hook iteration does.
pub(crate) fn sling_modules(nbt: &NbtCompound, env: &Environment) -> Vec<(crate::modifier_entries::Entry, Identifier, crate::sling_modules::Sling)> {
    if is_broken(nbt) { return Vec::new(); }
    with_tool_facts(nbt, env, |saved, facts| {
        let mut found = Vec::new();
        for (entry, modifier) in saved.iter() {
            for module in modifier.modules() {
                if let Some(sling) = module.sling(entry, facts) { found.push((entry.clone(), entry.id.clone(), sling.clone())); }
            }
        }
        found
    })
}

/// SlingForceModifierHook over the tool: each of the tool's sling force modules reshapes
/// the force in modifier order, filtered on its predicates against the modifier doing the
/// slinging (`sling`), the holder and the target.
pub(crate) fn sling_force(nbt: &NbtCompound, env: &Environment, entities: &crate::entity_facts::Entities, holder: &crate::sling_modules::LivingState, target: &crate::sling_modules::LivingState, sling: &Identifier, force: f32, multiplier: f32) -> f32 {
    let in_tag = |modifier: &Identifier, tag: &Identifier| env.modifiers.reverse_tags.get(modifier).is_some_and(|tags| tags.contains(tag));
    let view = crate::mining_speed::ToolView::with(nbt, *entities);
    with_tool_facts(nbt, env, |saved, facts| {
        let mut force = force;
        for (entry, modifier) in saved.iter() {
            for module in modifier.modules() {
                force = module.sling_force(entry, facts, &view, holder, target, sling, &in_tag, force, multiplier);
            }
        }
        force
    })
}

/// The power a sling spends (SlingModule.getPower): half the tool's conditional
/// projectile damage times its conditional velocity, for the holder `entities`.
pub(crate) fn sling_power(nbt: &NbtCompound, env: &Environment, entities: &crate::entity_facts::Entities) -> f32 {
    conditional_stat(nbt, env, Stat::ProjectileDamage, &|_| false, entities) / 2.0 * conditional_stat(nbt, env, Stat::Velocity, &|_| false, entities)
}

/// The draw speed a sling charges by (the conditional stat in SlingModule.getDrawtime).
pub(crate) fn draw_speed(nbt: &NbtCompound, env: &Environment, entities: &crate::entity_facts::Entities) -> f32 {
    conditional_stat(nbt, env, Stat::DrawSpeed, &|_| false, entities)
}

/// ModifierUtil.getInaccuracy: `3 * (1 / accuracy - 1)` over the conditional accuracy.
pub(crate) fn inaccuracy(nbt: &NbtCompound, env: &Environment, entities: &crate::entity_facts::Entities) -> f32 {
    3.0 * (1.0 / conditional_stat(nbt, env, Stat::Accuracy, &|_| false, entities) - 1.0)
}

/// The tool's first fireball module whose condition matches, with its entry and its
/// modifier's ID (FireballModule: one shot per use, from the first that fires).
pub(crate) fn fireball_module(nbt: &NbtCompound, env: &Environment) -> Option<(crate::modifier_entries::Entry, Identifier, crate::projectile_modules::Fireball)> {
    if is_broken(nbt) { return None; }
    with_tool_facts(nbt, env, |saved, facts| {
        for (entry, modifier) in saved.iter() {
            for module in modifier.modules() {
                if let Some(fireball) = module.fireball(entry, facts) { return Some((entry.clone(), entry.id.clone(), fireball.clone())); }
            }
        }
        None
    })
}

/// The infinity supply of a launcher: the first module in modifier order that fires for this shot (one that
/// skips standard ammo, or whose count the remaining durability cannot pay for, lets the next try, as the
/// composed bow ammo hook passes an empty result on), as the stack to fire, the durability one shot costs
/// and the modifier that owns the module (the cause of the wear). None when no module fires.
pub(crate) fn infinity(nbt: &NbtCompound, env: &Environment, standard_present: bool) -> Option<(pluto_lib::ItemSlot, i32, Identifier)> {
    let durability = current_durability(nbt);
    with_tool_facts(nbt, env, |saved, facts| {
        for (entry, modifier) in saved {
            for module in modifier.modules() {
                let Some(module) = module.infinity() else { continue; };
                if module.skips_standard(standard_present) { continue; }
                let count = module.count(durability);
                if count == 0 { continue; }
                let variant = (facts.persistent_string)(&entry.id);
                return Some((module.supply(count, &variant), module.usage(), entry.id.clone()));
            }
        }
        None
    })
}

/// What a tool's in-flight modules do to a projectile fired from it or as it, with every leveling value
/// already computed at the entry levels the tool carries (the profile is built per projectile, from the
/// stack that was fired). Read by `flight_hook` on the spawn, flight and impact tellings.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct FlightProfile {
    /// ArrowPierceModule: the pierce level to set on spawn.
    pub pierce: Option<u8>,
    /// ProjectileBounceModule instances, in modifier order: the modifier's id and its bounce limit.
    pub bounces: Vec<(String, i32)>,
    /// ProjectileGravityModule: ticks of flight without gravity (the last module's scheduled task wins,
    /// as the original's tasks share one id).
    pub gravity_delay: Option<u32>,
    /// ProjectileFuseModule: the particle to shed and the tick the fuse ends (the last module's task wins).
    pub fuse: Option<(String, u32)>,
    /// ProjectileExplosionModule instances: (radius at level, place fire, destroy blocks, efln bonus).
    pub explosions: Vec<(f32, bool, bool, f32)>,
    /// ProjectileAttractMobsModule instances: (radius, strength).
    pub attracts: Vec<(f32, f32)>,
    /// ProjectilePlaceGlowModule instances: (places on blocks, places on entities, launcher wear).
    pub glows: Vec<(bool, bool, i32)>,
    /// RestrictAngleModule: the fan size the shot's direction is clamped to.
    pub angle_directions: Option<u32>,
    /// The tool's loyalty level (the `tconstruct:loyalty` modifier), for a thrown tool's return.
    pub loyalty: i32,
    /// The tool's magnetic level (the `tconstruct:magnetic` modifier), for a thrown tool's item pull.
    pub magnet: i32,
    /// Whether the projectile carries the fins EFLN mark: set by a projectile-bound volatile
    /// flag on the fired stack (fins), or already present in its persistent data.
    pub efln: bool,
    /// Whether the projectile carries the enderference mark: set by a projectile-bound volatile
    /// flag on the fired stack (enderference). The original reads it on an entity hit to let the
    /// shot's hit hooks run on an enderman the enderference effect does not protect
    /// (ToolEvents.projectileHit); the impact telling carries neither the target's kind nor its
    /// effects, so the mark is carried and readable here but gates nothing yet.
    pub enderference: bool,
    /// Whether the tool carries enderporting (the `tconstruct:enderporting` modifier): where its
    /// projectile lands, the thrower trades places with what it hit.
    pub enderporting: bool,
    /// TippedModule: the potion the fired stack was tipped with (its persistent string
    /// under the modifier's ID, copied at the shot as onProjectileShoot copies it) and
    /// the entry's int effective level, which sets the hit's divisor.
    pub tipped: Option<(String, i32)>,
}

impl FlightProfile {
    /// Whether nothing in the profile changes a flight (no spawn, flight or impact work to do).
    pub(crate) fn is_empty(&self) -> bool {
        self.pierce.is_none() && self.bounces.is_empty() && self.gravity_delay.is_none() && self.fuse.is_none()
            && self.explosions.is_empty() && self.attracts.is_empty() && self.glows.is_empty() && self.angle_directions.is_none()
            && !self.enderporting && !self.enderference && self.tipped.is_none()
    }
}

/// The flight profile of the tool in `nbt`, from its saved modifiers.
pub(crate) fn flight_profile(nbt: &NbtCompound, env: &Environment) -> FlightProfile {
    let mut profile = with_tool_facts(nbt, env, |saved, facts| {
        let mut profile = FlightProfile::default();
        for (entry, modifier) in saved {
            match entry.id.to_string().as_str() {
                "tconstruct:loyalty" => profile.loyalty = entry.effective_level() as i32,
                "tconstruct:magnetic" => profile.magnet = entry.effective_level() as i32,
                "tconstruct:enderporting" => profile.enderporting = true,
                _ => {}
            }
            for module in modifier.modules() {
                if let Some(module) = module.arrow_pierce() { profile.pierce = Some(module.pierce(entry)); }
                if let Some(module) = module.projectile_bounce() { profile.bounces.push((entry.id.to_string(), module.limit(entry))); }
                if let Some(module) = module.projectile_gravity() { profile.gravity_delay = Some(module.delay(entry)); }
                if let Some(module) = module.projectile_fuse() { profile.fuse = Some((module.particle().to_owned(), module.ticks(entry))); }
                if let Some(module) = module.projectile_explosion() {
                    profile.explosions.push((module.radius(entry, false), module.place_fire(), module.destroy(), module.radius(entry, true) - module.radius(entry, false)));
                }
                if let Some(module) = module.projectile_attract() { profile.attracts.push((module.radius(entry), module.strength(entry))); }
                if let Some(module) = module.projectile_glow() { profile.glows.push((module.blocks(), module.entities(), module.tool_damage())); }
                if let Some(module) = module.restrict_angle() { profile.angle_directions = Some(module.directions(entry)); }
                // VolatileFlagModule's launch half: a flag bound to a projectile hook is written
                // into the launched projectile's persistent data, which the profile stands in for
                // here; the explosion form and the enderference mark are the two readers.
                if let Some(flag) = module.projectile_flag(entry, facts) {
                    match flag.to_string().as_str() {
                        "tconstruct:efln" => profile.efln = true,
                        "tconstruct:enderference" => profile.enderference = true,
                        _ => {}
                    }
                }
                // TippedModule: the potion is read from the persistent data below; the
                // modifier's ID stands in as the key until then.
                if module.is_tipped() { profile.tipped = Some((entry.id.to_string(), entry.int_effective_level())); }
            }
        }
        profile
    });
    if let Some(Nbt::Compound(persistent)) = nbt.get(TAG_PERSISTENT) {
        profile.efln = matches!(persistent.get("tconstruct:efln"), Some(Nbt::Byte(1)) | Some(Nbt::Int(1)));
        // The tipped potion: the persistent string under the modifier's own ID, as the
        // tipping casting recipes leave it; no string, no tip.
        if let Some((key, level)) = profile.tipped.take() {
            profile.tipped = match persistent.get(&key) {
                Some(Nbt::String(potion)) if !potion.is_empty() => Some((potion.clone(), level)),
                _ => None,
            };
        }
    }
    profile
}

/// Whether the tool is thrown on release of a charged use (ThrowingModule) and is not broken.
pub(crate) fn throwing(nbt: &NbtCompound, env: &Environment) -> bool {
    !is_broken(nbt) && with_tool_facts(nbt, env, |saved, _facts| saved.iter().any(|(_entry, modifier)| modifier.modules().iter().any(|module| module.is_throwing())))
}

/// Whether the tool carries the enderporting module (EnderportingModifier) and is not broken.
pub(crate) fn enderporting(nbt: &NbtCompound, env: &Environment) -> bool {
    !is_broken(nbt) && with_tool_facts(nbt, env, |saved, _facts| saved.iter().any(|(_entry, modifier)| modifier.modules().iter().any(|module| module.is_enderporting())))
}

/// Whether the tool carries the harvest module (HarvestModule) and is not broken.
pub(crate) fn harvests(nbt: &NbtCompound, env: &Environment) -> bool {
    !is_broken(nbt) && with_tool_facts(nbt, env, |saved, _facts| saved.iter().any(|(_entry, modifier)| modifier.modules().iter().any(|module| module.is_harvest())))
}

/// The stack edits of wearing this tool by `amount` (ToolDamageUtil.damageTool, as the repair and
/// swap flows apply it directly): the damage value, and the broken mark when the wear breaks it.
/// Empty when the wear does not land (a broken or unbreakable tool, or nothing to wear).
pub(crate) fn damage_edits(nbt: &NbtCompound, amount: i32) -> Vec<pluto_lib::StackDataEdit> {
    let mut work = nbt.clone();
    let damaged = crate::tool_damage::damage_tool(&mut work, amount, true, true);
    if damaged.applied <= 0 { return Vec::new(); }
    let mut edits = vec![pluto_lib::StackDataEdit { path: vec![TAG_DAMAGE.to_string()], value: Some(Nbt::Int(crate::tool_stack::damage_raw(&work))) }];
    if damaged.broke {
        crate::tool_damage::break_tool(&mut work);
        edits.push(pluto_lib::StackDataEdit { path: vec![TAG_BROKEN.to_string()], value: Some(Nbt::Byte(1)) });
    }
    edits
}

/// The loot of a block broken with this tool after its loot to capacity modules ate what they take (each in modifier order, on what the one
/// before left), and the edits of the persistent data that fill their bars. `in_tag(item, tag)` answers item tag membership.
pub(crate) fn loot_capacity(nbt: &NbtCompound, env: &Environment, loot: &mut Vec<pluto_lib::ItemSlot>, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool, rolls: &mut dyn FnMut() -> f32) -> Vec<pluto_lib::StackDataEdit> {
    with_tool_facts(nbt, env, |saved, facts| {
        let mut work = nbt.clone();
        for (entry, modifier) in saved {
            for module in modifier.loot_capacity_modules() {
                let Some(fill) = module.loot_capacity(entry, facts) else { continue };
                let added = fill.eat(entry, loot, in_tag, rolls);
                if added <= 0 { continue; }
                if let Some(target) = crate::capacity::Target::find(saved, entry, fill.owner()) { target.grant(&mut work, env, added); }
            }
        }
        crate::capacity::persistent_edits(nbt, &work)
    })
}

/// AutosmeltModule.processLoot over the tool's autosmelt modules, in modifier order: each drop a
/// cooking recipe of one of the module's types takes becomes the cooked result, its count scaled
/// by the input's; past the first level, each input item rolls the extra drop chance for one more
/// result, unless the result is plus-blacklisted. A blacklisted drop is never converted. The
/// result is a plain stack of the result item: the recipe result's own components are not carried
/// by the cooking lookup. `in_tag(item, tag)` answers item tag membership; `rolls` gives uniform
/// values in [0, 1).
pub(crate) fn autosmelt_loot(nbt: &NbtCompound, env: &Environment, loot: &mut Vec<pluto_lib::ItemSlot>, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool, rolls: &mut dyn FnMut() -> f32) {
    let blacklist = Identifier::parse("tconstruct:autosmelt_blacklist").expect("valid tag");
    let plus_blacklist = Identifier::parse("tconstruct:autosmelt_plus_blacklist").expect("valid tag");
    with_tool_facts(nbt, env, |saved, _facts| {
        for (entry, modifier) in saved {
            for module in modifier.modules() {
                let Some((types, chance)) = module.autosmelt() else { continue; };
                for stack in loot.iter_mut() {
                    if in_tag(&stack.item, &blacklist) { continue; }
                    let Some(recipe) = types.iter().find_map(|kind| pluto_lib::recipe_api_v1::cooking_recipe(kind, &stack.item.to_string())) else { continue; };
                    let Ok(result) = Identifier::parse(&recipe.result) else { continue; };
                    let input_count = stack.count;
                    let mut count = recipe.result_count * input_count;
                    if entry.level > 1 && chance > 0.0 && !in_tag(&result, &plus_blacklist) {
                        let extra = input_count as i32 * (entry.level - 1);
                        let bonus = if chance >= 1.0 { extra } else { (0..extra).filter(|_| rolls() < chance).count() as i32 };
                        count += bonus as u32;
                    }
                    *stack = pluto_lib::ItemSlot::new(result, count);
                }
            }
        }
    });
}

/// SmeltingModule.cookItems over the tool's smelting modules: `amount` of heat (a hit's damage,
/// a harvested count, a shot's power) times each module's multiplier is the cooking power every
/// stored input stack is measured against: a stack whose remaining time runs out gives one item
/// to the output inventory (the same slot), and starts its next item at the recipe's full time.
/// Mutates `nbt` (the tool's custom data) in place; returns how many items finished cooking.
/// Reductions against the original: a finished item's experience and sound are not granted (no
/// port action awards experience orbs or plays a sound at the holder), and the output slot's cap
/// is the module's slot limit alone (an item's own max stack size is not reported).
pub(crate) fn cook_smelting(nbt: &mut NbtCompound, env: &Environment, amount: f32) -> u32 {
    if amount < 0.0 { return 0; }
    let plans = with_tool_facts(nbt, env, |saved, facts| {
        saved.iter().flat_map(|(entry, modifier)| modifier.modules().iter().filter_map(|module| module.smelting_parts(entry, facts))
            .map(|(input, output, kind, multiplier)| (input, output, kind.to_owned(), multiplier)).collect::<Vec<_>>()).collect::<Vec<_>>()
    });
    let mut cooked = 0;
    for (input, output, recipe_type, multiplier) in plans {
        if !input.is_active() { continue; }
        let power = amount * multiplier;
        let output_inventory = crate::tool_inventory::ToolInventory::new(vec![output]);
        with_persistent(nbt, |persistent| {
            let mut entries = input.entries(persistent);
            let mut changed = false;
            let mut index = 0;
            while index < entries.len() {
                let slot = entries[index].0;
                let Some(mut stack) = crate::tool_inventory::read_stack(&entries[index].1) else { index += 1; continue; };
                let mut time = entries[index].1.get("tic_remaining_time").and_then(Nbt::as_i64).unwrap_or(0) as i32;
                let mut recipe = None;
                if time == 0 {
                    recipe = pluto_lib::recipe_api_v1::cooking_recipe(&recipe_type, &stack.item.to_string());
                    time = recipe.as_ref().map_or(-1, |recipe| recipe.time as i32);
                    entries[index].1.insert("tic_remaining_time".into(), Nbt::Int(time));
                    changed = true;
                }
                if time >= 0 {
                    if time as f32 > power {
                        entries[index].1.insert("tic_remaining_time".into(), Nbt::Int((time as f32 - power) as i32));
                        changed = true;
                    } else {
                        // The output slot this input slot cooks into: a full one freezes the item at 1 tick left.
                        let current = output_inventory.stack(persistent, slot);
                        let mut max_size = 0;
                        if let Some(current) = &current {
                            max_size = output_inventory.slot_limit(slot);
                            if current.count as i32 >= max_size {
                                entries[index].1.insert("tic_remaining_time".into(), Nbt::Int(1));
                                changed = true;
                                index += 1;
                                continue;
                            }
                        }
                        if recipe.is_none() { recipe = pluto_lib::recipe_api_v1::cooking_recipe(&recipe_type, &stack.item.to_string()); }
                        match recipe {
                            None => {
                                entries[index].1.insert("tic_remaining_time".into(), Nbt::Int(-1));
                                changed = true;
                            }
                            Some(recipe) => {
                                let Ok(result_id) = Identifier::parse(&recipe.result) else { index += 1; continue; };
                                let mut result = pluto_lib::ItemSlot::new(result_id, recipe.result_count);
                                if max_size == 0 { max_size = output_inventory.slot_limit(slot); }
                                let blocked = result.count as i32 + current.as_ref().map_or(0, |current| current.count as i32) > max_size
                                    || current.as_ref().is_some_and(|current| !crate::tool_inventory::same_stack(current, &result));
                                if blocked {
                                    entries[index].1.insert("tic_remaining_time".into(), Nbt::Int(1));
                                    changed = true;
                                } else {
                                    if let Some(mut current) = current { current.count += result.count; result = current; }
                                    output_inventory.set_stack(persistent, slot, Some(&result));
                                    cooked += 1;
                                    changed = true;
                                    stack.count -= 1;
                                    if stack.count == 0 { entries.remove(index); continue; }
                                    let mut rewritten = crate::tool_inventory::write_stack(&stack, slot);
                                    rewritten.insert("tic_remaining_time".into(), Nbt::Int(recipe.time as i32));
                                    entries[index].1 = rewritten;
                                }
                            }
                        }
                    }
                }
                index += 1;
            }
            if changed { input.set_entries(persistent, entries); }
        });
    }
    cooked
}

/// cook_smelting as stack edits, for the flows that land their changes as actions on the holder.
pub(crate) fn smelting_edits(nbt: &NbtCompound, env: &Environment, amount: f32) -> Vec<pluto_lib::StackDataEdit> {
    let mut work = nbt.clone();
    cook_smelting(&mut work, env, amount);
    crate::capacity::persistent_edits(nbt, &work)
}

/// The damage a worn armor piece lets through after its damage to capacity modules took what their bars hold (in priority order, highest
/// first), and the edits of the piece's persistent data that fill the bars.
pub(crate) fn damage_capacity(nbt: &NbtCompound, env: &Environment, source: &crate::protection::DamageSource, damage: f32) -> (f32, Vec<pluto_lib::StackDataEdit>) {
    with_tool_facts(nbt, env, |saved, facts| {
        let mut order: Vec<usize> = (0..saved.len()).collect();
        order.sort_by_key(|&index| (std::cmp::Reverse(saved[index].1.priority), index));
        let mut work = nbt.clone();
        let mut damage = damage;
        for index in order {
            let (entry, modifier) = &saved[index];
            for module in modifier.damage_capacity_modules() {
                let Some(fill) = module.damage_capacity(entry, facts, source) else { continue };
                if let Some(target) = crate::capacity::Target::find(saved, entry, fill.owner()) { damage = fill.absorb(&mut work, &target, entry, damage); }
            }
        }
        (damage, crate::capacity::persistent_edits(nbt, &work))
    })
}

/// The actions a worn armor piece does to the mob that hit its wearer (fire, freezing, knockback), in modifier order. `in_tag` answers the
/// entity type tags and the pseudo state tags of the attacking mob; `rolls` gives uniform values in [0, 1).
pub(crate) fn counter_actions(nbt: &NbtCompound, env: &Environment, in_tag: &dyn Fn(&Identifier) -> bool, rolls: &mut dyn FnMut() -> f32) -> Vec<pluto_lib::EntityAction> {
    with_tool_facts(nbt, env, |saved, facts| {
        let mut actions = Vec::new();
        for (entry, modifier) in saved {
            for module in modifier.counter_action_modules() {
                if let Some(action) = module.counter_action(entry, facts, in_tag, rolls) { actions.push(action); }
            }
        }
        actions
    })
}

/// The mob effects a worn armor piece puts on the mob that hit its wearer, in modifier order. `source` is the hit's damage; `in_tag`
/// answers the entity type tags and the pseudo state tags of the attacking mob; `rolls` gives uniform values in [0, 1).
pub(crate) fn counter_effects(nbt: &NbtCompound, env: &Environment, source: &crate::protection::DamageSource, in_tag: &dyn Fn(&Identifier) -> bool, rolls: &mut dyn FnMut() -> f32) -> Vec<(crate::weapon_effect::Whom, crate::weapon_effect::Grant)> {
    with_tool_facts(nbt, env, |saved, facts| {
        let mut grants = Vec::new();
        for (entry, modifier) in saved {
            for module in modifier.counter_effect_modules() {
                if let Some(grant) = module.counter_effect(entry, facts, source, in_tag, rolls) { grants.push(grant); }
            }
        }
        grants
    })
}

/// The mob effects a worn armor piece puts on the mob its wearer hit in melee, in modifier order. `source` is the hit's damage; `in_tag`
/// answers the entity type tags and the pseudo state tags (of the mob hit, and of the wearer for the holder); `rolls` gives uniform values in [0, 1).
pub(crate) fn armor_attack_effects(nbt: &NbtCompound, env: &Environment, source: &crate::protection::DamageSource, in_tag: &dyn Fn(&Identifier) -> bool, rolls: &mut dyn FnMut() -> f32) -> Vec<crate::weapon_effect::Grant> {
    with_tool_facts(nbt, env, |saved, facts| {
        let mut grants = Vec::new();
        for (entry, modifier) in saved {
            for module in modifier.armor_attack_effect_modules() {
                if let Some(grant) = module.armor_attack_effect(entry, facts, source, in_tag, rolls) { grants.push(grant); }
            }
        }
        grants
    })
}

/// ModifierRemovalHook.onRemoved(original, updated): every modifier on the original that the updated
/// tool no longer has clears the persistent data its modules own. Traits count, as in the original.
pub(crate) fn on_removed(original: &NbtCompound, updated: &mut NbtCompound, env: &Environment) {
    let remaining = List::from_nbt(updated.get(TAG_MODIFIERS));
    let mut persistent = match updated.get(TAG_PERSISTENT) { Some(Nbt::Compound(data)) => data.clone(), _ => return };
    let before = persistent.clone();
    for (entry, modifier) in saved_modifiers(original, env.modifiers) {
        if remaining.level(&entry.id) == 0 { modifier.on_removed(&mut persistent); }
    }
    if persistent != before { updated.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent)); }
}

/// ToolStack.tryValidate: a negative free slot count, then each recipe-added modifier's requirements.
/// The error is a translation key, followed by a space and the slot's display key for the slot error.
pub(crate) fn try_validate(nbt: &NbtCompound, env: &Environment) -> Option<String> {
    const KNOWN: [&str; 4] = ["upgrades", "defense", "abilities", "souls"];
    let compound = |tag: &str| match nbt.get(tag) { Some(Nbt::Compound(data)) => data.clone(), _ => NbtCompound::new() };
    let (volatile, persistent) = (compound(TAG_VOLATILE), compound(TAG_PERSISTENT));
    let mut names: Vec<String> = KNOWN.iter().map(|name| (*name).to_owned()).collect();
    for key in volatile.keys().chain(persistent.keys()).filter(|key| !key.contains(':')) {
        if !names.contains(key) { names.push(key.clone()); }
    }
    for name in names {
        let free = crate::modifier_entries::nbt_int(persistent.get(&name)).wrapping_add(crate::modifier_entries::nbt_int(volatile.get(&name)));
        if free < 0 { return Some(format!("recipe.tconstruct.modifier.validate_slots slot_type.tconstruct.{name}")); }
    }
    // Stacks that no longer fit the tool's slots stop the change (the move itself is validate_inventory).
    let mut checked = nbt.clone();
    if let Err(error) = validate_inventory(&mut checked, env) { return Some(error.to_owned()); }
    let upgrades = List::from_nbt(nbt.get(TAG_UPGRADES));
    with_tool_facts(nbt, env, |_, facts| {
        upgrades.entries().iter().find_map(|entry| env.modifiers.get(&entry.id).and_then(|modifier| modifier.validate_upgrade(entry, facts)))
    })
}

/// The tool's item handler: every inventory module of its modifiers, the last modifier first (ToolInventoryCapability).
pub(crate) fn inventory(nbt: &NbtCompound, env: &Environment) -> crate::tool_inventory::ToolInventory {
    // ModifiableBowItem.isBallista, which a quiver's validity asks about: the ballistas tag and the flag set.
    let ballista = match (env.item, env.item_tags) {
        (Some(item), Some(tags)) => tags.get_tag(&Identifier::parse("tconstruct:modifiable/ranged/ballistas").expect("valid tag")).is_some_and(|members| members.contains(item)),
        _ => false,
    } && volatile_flag(nbt, "tconstruct:ballista");
    with_tool_facts(nbt, env, |saved, facts| {
        let mut parts: Vec<crate::tool_inventory::Part> = saved.iter().rev().flat_map(|(entry, modifier)| modifier.inventory_parts(entry, facts)).collect();
        if ballista { for part in &mut parts { if part.is_quiver() { part.note_ballista(); } } }
        crate::tool_inventory::ToolInventory::new(parts)
    })
}

/// The slots the tool's volatile data counts (ToolInventoryCapability.TOTAL_SLOTS); the capability exists only above zero.
/// BowAmmoModifierHook.getAmmo over the tool's own modifiers, in modifier order: the first quiver module that
/// finds ammunition supplies one of it, already shrunk out of the tool's inventory in `nbt` (findAmmo and
/// shrinkAmmo run in one step here, as the ammo event's contract has the callback consume its own source).
/// `vanilla` is the stack the game picked itself (BulkQuiverModule's standardAmmo): a bulk quiver that checks
/// the standard arrows is a backup and stays silent while it is non-empty. A trick quiver's selection cycling
/// (InventorySelectionModule.selectNext) is not run here: it rides the tool's use, and the use events report
/// neither the hand nor the player's crouch state the cycling keys on (see quiver_hook).
pub(crate) fn draw_quiver_ammo(nbt: &mut NbtCompound, env: &Environment, vanilla: Option<&pluto_lib::ItemSlot>, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<pluto_lib::ItemSlot> {
    let ballista = match (env.item, env.item_tags) {
        (Some(item), Some(tags)) => tags.get_tag(&Identifier::parse("tconstruct:modifiable/ranged/ballistas").expect("valid tag")).is_some_and(|members| members.contains(item)),
        _ => false,
    } && volatile_flag(nbt, "tconstruct:ballista");
    let draws = with_tool_facts(nbt, env, |saved, facts| {
        saved.iter().filter_map(|(entry, modifier)| {
            let draw = modifier.quiver_draw()?;
            let mut parts = modifier.inventory_parts(entry, facts);
            if ballista { for part in &mut parts { if part.is_quiver() { part.note_ballista(); } } }
            Some((draw, crate::tool_inventory::ToolInventory::new(parts)))
        }).collect::<Vec<_>>()
    });
    for (draw, inventory) in draws {
        let drawn = with_persistent(nbt, |persistent| {
            let found = match draw {
                crate::modifier_modules::QuiverDraw::Bulk { check_standard_arrows } => {
                    if check_standard_arrows && vanilla.is_some() { return None; }
                    let (stack, slot) = inventory.find_ammo(persistent, in_tag)?;
                    // BulkQuiverModule records the drawn slot for its shrink, which happens below in the same step.
                    persistent.insert("tconstruct:quiver_last_selected".into(), Nbt::Int(slot as i32));
                    Some((stack, slot))
                }
                crate::modifier_modules::QuiverDraw::Trick => {
                    let selected = crate::modifier_entries::nbt_int(persistent.get("tconstruct:trick_quiver_selected"));
                    if selected < 0 { return None; }
                    inventory.stack(persistent, selected as usize).map(|stack| (stack, selected as usize))
                }
            };
            found.map(|(stack, slot)| {
                let mut rest = stack.clone();
                rest.count -= 1;
                inventory.set_stack(persistent, slot, (rest.count > 0).then_some(&rest));
                let mut supplied = stack;
                supplied.count = 1;
                supplied
            })
        });
        if drawn.is_some() { return drawn; }
    }
    None
}

/// One step of an armor keybind interact (ArmorInteractModifierHook), in the merger's order: the strap,
/// belt and minimap modules of every entry in modifier order first (their module priority is the
/// default), then the inventory menu modules (InventoryMenuModule.getPriority is 75, so a menu opens
/// only when no strap, belt or minimap module answered the key).
pub(crate) enum InteractPlan {
    Strap { keys: Vec<crate::tool_inventory::MenuKey>, inventory: crate::tool_inventory::ToolInventory },
    Belt { keys: Vec<crate::tool_inventory::MenuKey>, inventory: crate::tool_inventory::ToolInventory },
    /// MinimapModule: the keybind cycles the minimap inventory's selected map (its own keys are the
    /// tooltip keys it answers, decided in armor_interact_hook's `interact`).
    MinimapSelect { inventory: crate::tool_inventory::ToolInventory },
    Menu { key: crate::tool_inventory::MenuKey },
}

/// The interact plan of a worn piece: each strap or belt step carries its own modifier's inventory, as the
/// original's modules read their own entry's inventory capability.
pub(crate) fn armor_interact_plan(nbt: &NbtCompound, env: &Environment) -> Vec<InteractPlan> {
    with_tool_facts(nbt, env, |saved, facts| {
        let mut plan = Vec::new();
        for (entry, modifier) in saved {
            let mut inventory: Option<crate::tool_inventory::ToolInventory> = None;
            for module in modifier.modules() {
                let step = match module {
                    crate::modifier_modules::Module::ShieldStrap { keys } => Some(InteractPlan::Strap { keys: keys.clone(), inventory: inventory.get_or_insert_with(|| crate::tool_inventory::ToolInventory::new(modifier.inventory_parts(entry, facts))).clone() }),
                    crate::modifier_modules::Module::ToolBelt { keys } => Some(InteractPlan::Belt { keys: keys.clone(), inventory: inventory.get_or_insert_with(|| crate::tool_inventory::ToolInventory::new(modifier.inventory_parts(entry, facts))).clone() }),
                    crate::modifier_modules::Module::Minimap => Some(InteractPlan::MinimapSelect { inventory: inventory.get_or_insert_with(|| crate::tool_inventory::ToolInventory::new(modifier.inventory_parts(entry, facts))).clone() }),
                    _ => None,
                };
                if let Some(step) = step { plan.push(step); }
            }
        }
        for (_, modifier) in saved {
            for key in modifier.inventory_menu_keys() { plan.push(InteractPlan::Menu { key }); }
        }
        plan
    })
}

pub(crate) fn inventory_slots(nbt: &NbtCompound) -> i32 {
    match nbt.get(TAG_VOLATILE) { Some(Nbt::Compound(volatile)) => crate::modifier_entries::nbt_int(volatile.get(crate::tool_inventory::TOTAL_SLOTS)), _ => 0 }
}

/// Runs `edit` over the tool's persistent data (the stored stacks live there) and writes it back.
pub(crate) fn with_persistent<R>(nbt: &mut NbtCompound, edit: impl FnOnce(&mut NbtCompound) -> R) -> R {
    let mut persistent = match nbt.get(TAG_PERSISTENT) { Some(Nbt::Compound(data)) => data.clone(), _ => NbtCompound::new() };
    let result = edit(&mut persistent);
    if persistent.is_empty() { if nbt.contains_key(TAG_PERSISTENT) && !matches!(nbt.get(TAG_PERSISTENT), Some(Nbt::Compound(data)) if data.is_empty()) { nbt.remove(TAG_PERSISTENT); } }
    else { nbt.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent)); }
    result
}

/// InventoryModule.validate over the tool: stacks above a module's slots move down; an error key when they cannot.
pub(crate) fn validate_inventory(nbt: &mut NbtCompound, env: &Environment) -> Result<(), &'static str> {
    let handler = inventory(nbt, env);
    with_persistent(nbt, |persistent| handler.validate(persistent))
}

/// ToolDamageUtil.damage's modifier loop: each modifier may cancel part of the damage; once nothing
/// is left the tool takes none. `cause` is the damaging modifier, None for ordinary use.
pub(crate) fn reduce_damage(nbt: &NbtCompound, env: &Environment, cause: Option<&Identifier>, mut amount: i32, random: &mut dyn FnMut() -> f32) -> i32 {
    let modifier_in_tag = |modifier: &Identifier, tag: &Identifier| env.modifiers.reverse_tags.get(modifier).is_some_and(|tags| tags.contains(tag));
    with_tool_facts(nbt, env, |saved, facts| {
        for (entry, modifier) in saved {
            amount = modifier.reduce_damage(entry, facts, nbt, cause, &modifier_in_tag, amount, random);
            if amount <= 0 { return 0; }
        }
        amount
    })
}

/// RepairFactorModifierHook.getRepairFactor: composes every modifier, and stops at zero.
pub(crate) fn repair_factor(nbt: &NbtCompound, env: &Environment, mut factor: f32) -> f32 {
    with_tool_facts(nbt, env, |saved, facts| {
        for (entry, modifier) in saved {
            factor = modifier.repair_factor(entry, facts, factor);
            if factor <= 0.0 { return 0.0; }
        }
        factor
    })
}

/// CraftCountModifierHook.maxStackSize: how many of the tool one craft produces. A tool whose item stacks to
/// one never runs the hooks; the composed chain multiplies in modifier order and stops at zero or below,
/// and the result never exceeds the item's own maximum.
pub(crate) fn max_stack_size(nbt: &NbtCompound, env: &Environment, item_max: i32, count: i32) -> i32 {
    if item_max == 1 { return 1; }
    let amount = with_tool_facts(nbt, env, |saved, facts| {
        let mut amount = count as f32;
        for (entry, modifier) in saved {
            amount = modifier.craft_count(entry, facts, amount);
            if amount <= 0.0 { return 0.0; }
        }
        amount
    });
    if amount <= 0.0 { 0 } else { (amount as i32).min(item_max) }
}

/// MaterialRepairModifierHook over the tool's modifiers: whether any of them repairs with `material`.
pub(crate) fn modifier_is_repair_material(nbt: &NbtCompound, env: &Environment, material: &Identifier) -> bool {
    with_tool_facts(nbt, env, |saved, facts| saved.iter().any(|(entry, modifier)| modifier.is_repair_material(entry, facts, material)))
}

/// The repair a material stack gives through the tool's modifiers alone (the caller takes the larger of this
/// and the definition's own amount, as MaterialRepairToolHook does).
pub(crate) fn modifier_repair_amount(nbt: &NbtCompound, env: &Environment, material: &Identifier) -> i32 {
    let durability_of = |material: &Identifier, stat: &Identifier| crate::tool_definitions::repair_durability_of(env.materials, material, stat);
    with_tool_facts(nbt, env, |saved, facts| saved.iter().map(|(entry, modifier)| modifier.material_repair_amount(entry, facts, material, &durability_of)).sum())
}

/// The materials the tool's modifiers add as repair materials (MaterialRepairModifierHook.addRepairMaterials).
pub(crate) fn modifier_repair_materials(nbt: &NbtCompound, env: &Environment) -> Vec<Identifier> {
    with_tool_facts(nbt, env, |saved, _facts| saved.iter().flat_map(|(_, modifier)| modifier.repair_materials().cloned()).collect())
}

/// VolatileFloatModule on a launched projectile: each module's value added onto the projectile's persistent
/// data under its flag. No live caller yet: the launch event reports no stack to write to (the projectile
/// power helpers above are in the same position).
pub(crate) fn launch_volatile_floats(nbt: &NbtCompound, env: &Environment, projectile: &mut NbtCompound) {
    let floats = with_tool_facts(nbt, env, |saved, facts| saved.iter().flat_map(|(entry, modifier)| modifier.projectile_floats(entry, facts)).collect::<Vec<_>>());
    for (flag, amount) in floats {
        let previous = projectile.get(&flag.to_string()).and_then(Nbt::as_f64).unwrap_or(0.0) as f32;
        projectile.insert(flag.to_string(), Nbt::Float(previous + amount));
    }
}

/// UpdateHealthModule over a worn piece: the health delta for a change in `slot`, going on or coming off.
pub(crate) fn update_health_delta(nbt: &NbtCompound, env: &Environment, slot: &str, equipped: bool) -> f32 {
    with_tool_facts(nbt, env, |saved, facts| saved.iter().map(|(entry, modifier)| modifier.update_health(entry, facts, slot, equipped)).sum())
}

/// StatsNBT.getInt(DURABILITY): default 1 when the stat is absent.
pub(crate) fn durability(nbt: &NbtCompound) -> i32 {
    let Some(Nbt::Compound(stats)) = nbt.get(TAG_STATS) else { return 1; };
    match stats.get(&format!("tconstruct:{}", Stat::Durability.path())) {
        Some(Nbt::Float(value)) => *value as i32,
        Some(Nbt::Double(value)) => *value as i32,
        Some(value) => value.as_i64().map(|v| v as i32).unwrap_or(1),
        None => 1,
    }
}
/// A stored float stat, or the stat's registered default when absent.
pub(crate) fn stat(nbt: &NbtCompound, stat: Stat) -> f32 {
    if let Some(Nbt::Compound(stats)) = nbt.get(TAG_STATS) {
        match stats.get(&format!("tconstruct:{}", stat.path())) {
            Some(Nbt::Float(value)) => return *value,
            Some(Nbt::Double(value)) => return *value as f32,
            Some(value) => if let Some(value) = value.as_i64() { return value as f32; },
            None => {}
        }
    }
    stat.default_value()
}
/// The per-stack `minecraft:tool` component to store on the stack after a rebuild or damage
/// change; the caller writes it to `ItemSlot::tool`. None for tools with no mining module.
pub(crate) fn mining_component(nbt: &NbtCompound, definition: &Data) -> Result<Option<NbtCompound>, String> {
    definition.mining_component(is_broken(nbt), stat(nbt, Stat::MiningSpeed), &harvest_tier(nbt))
}
pub(crate) fn damage_raw(nbt: &NbtCompound) -> i32 { crate::modifier_entries::nbt_int(nbt.get(TAG_DAMAGE)) }
pub(crate) fn is_broken(nbt: &NbtCompound) -> bool { boolean(nbt, TAG_BROKEN) }
/// ToolStack.setDamage: reaching durability breaks the tool and caps stored damage there.
pub(crate) fn set_damage(nbt: &mut NbtCompound, damage: i32) {
    let durability = durability(nbt);
    let damage = if damage >= durability { nbt.insert(TAG_BROKEN.into(), Nbt::Byte(1)); durability.max(0) }
        else { nbt.insert(TAG_BROKEN.into(), Nbt::Byte(0)); damage };
    nbt.insert(TAG_DAMAGE.into(), Nbt::Int(damage));
}
/// ToolStack.getDamage: broken tools report full damage; otherwise stay one below durability.
pub(crate) fn damage(nbt: &NbtCompound) -> i32 {
    let durability = durability(nbt);
    if is_broken(nbt) { durability } else { damage_raw(nbt).min(durability.wrapping_sub(1)) }
}
pub(crate) fn current_durability(nbt: &NbtCompound) -> i32 {
    if is_broken(nbt) { 0 } else { durability(nbt).wrapping_sub(damage_raw(nbt)).max(0) }
}
/// The stored harvest tier, defaulting to wood like HarvestTiers.minTier.
pub(crate) fn harvest_tier(nbt: &NbtCompound) -> Identifier {
    if let Some(Nbt::Compound(stats)) = nbt.get(TAG_STATS) {
        if let Some(Nbt::String(tier)) = stats.get(TIER_KEY) { if let Ok(tier) = Identifier::parse(tier) { return tier; } }
    }
    Identifier::parse("minecraft:wood").unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    struct Always;
    impl crate::materials::Conditions for Always {
        type Parsed = ();
        fn decode(&self, _: &serde_json::Value) -> Result<(), String> { Ok(()) }
        fn test(&self, _: &()) -> Result<bool, String> { Ok(true) }
    }
    fn environment<'a>(data: &'a Data, materials: &'a Materials, registry: &'a Registry) -> Environment<'a> {
        Environment { definition: Some(data), has_materials: false, materials, modifiers: registry, item_tags_loaded: true, item: None, item_tags: None }
    }
    fn loaded_materials() -> Materials { let mut materials = Materials::default(); materials.server_loaded = true; materials }
    #[test]
    fn rebuild_persists_and_lowers_damage_when_durability_shrinks() {
        let mut registry = Registry::default();
        registry.reload(&[(Identifier::parse("test:rare").unwrap(), json!({"modules":[{"type":"tconstruct:rarity","rarity":"epic"}]}))], &Always);
        let materials = loaded_materials();
        let strong = Data::decode(&json!({"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:durability":100}}]})).unwrap();
        let mut nbt = NbtCompound::new();
        nbt.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse("test:rare").unwrap(), 2).unwrap().to_nbt());
        assert!(!is_initialized(&nbt));
        assert!(rebuild_stats(&mut nbt, &environment(&strong, &materials, &registry)).unwrap());
        assert!(is_initialized(&nbt));
        assert_eq!(durability(&nbt), 100);
        assert_eq!(nbt[TAG_VOLATILE], Nbt::Compound([("tconstruct:rarity".to_owned(), Nbt::Int(3))].into_iter().collect()));
        let Nbt::List(modifiers) = &nbt[TAG_MODIFIERS] else { panic!() };
        assert_eq!(modifiers.len(), 1);
        set_damage(&mut nbt, 60);
        assert_eq!((damage(&nbt), current_durability(&nbt), is_broken(&nbt)), (60, 40, false));
        // A weaker definition lowers the max below stored damage: capped at the new maximum and broken.
        let weak = Data::decode(&json!({"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:durability":50}}]})).unwrap();
        assert!(rebuild_stats(&mut nbt, &environment(&weak, &materials, &registry)).unwrap());
        assert_eq!((damage_raw(&nbt), damage(&nbt), current_durability(&nbt), is_broken(&nbt)), (50, 50, 0, true));
        // Healing below durability clears the broken flag.
        set_damage(&mut nbt, 10);
        assert!(!is_broken(&nbt));
    }
    #[test]
    fn flight_profile_carries_the_projectile_bound_flags() {
        let mut registry = Registry::default();
        registry.reload(&[
            (Identifier::parse("test:fins").unwrap(), json!({"modules":[{"type":"tconstruct:volatile_flag","flag":"tconstruct:efln","hooks":["tconstruct:projectile_launch","tconstruct:projectile_shot"]}]})),
            (Identifier::parse("test:enderference").unwrap(), json!({"modules":[{"type":"tconstruct:volatile_flag","flag":"tconstruct:enderference","hooks":["tconstruct:projectile_launch","tconstruct:projectile_shot","tconstruct:projectile_thrown"]}]})),
            (Identifier::parse("test:explosive").unwrap(), json!({"modules":[{"type":"tconstruct:projectile_explosion","block_interaction":"destroy","efln_bonus":0.5,"knockback":{"flat":1.0},"place_fire":false,"radius":{"each_level":1.0,"flat":1.0},"hooks":["tconstruct:projectile_hit"]}]})),
            (Identifier::parse("test:plain").unwrap(), json!({})),
        ], &Always);
        let materials = loaded_materials();
        let data = Data::default();
        let env = environment(&data, &materials, &registry);
        let mut nbt = NbtCompound::new();
        let list = List::default()
            .with_modifier(Identifier::parse("test:fins").unwrap(), 1).unwrap()
            .with_modifier(Identifier::parse("test:enderference").unwrap(), 1).unwrap()
            .with_modifier(Identifier::parse("test:explosive").unwrap(), 1).unwrap();
        nbt.insert(TAG_MODIFIERS.into(), list.to_nbt());
        let profile = flight_profile(&nbt, &env);
        // Launching with fins bound marks the projectile efln: the explosion consumer adds the
        // module's efln bonus to the radius for a marked projectile (the EFLN form). The
        // enderference mark rides the profile the same way.
        assert!(profile.efln, "fins marks the launched projectile");
        assert!(profile.enderference, "enderference marks the launched projectile");
        assert_eq!(profile.explosions.len(), 1);
        assert_eq!(profile.explosions[0].3, 0.5, "the efln bonus the marked form adds");
        assert!(!profile.is_empty());
        // A stack with none of them carries neither mark.
        let mut bare = NbtCompound::new();
        bare.insert(TAG_MODIFIERS.into(), List::default().with_modifier(Identifier::parse("test:plain").unwrap(), 1).unwrap().to_nbt());
        let profile = flight_profile(&bare, &env);
        assert!(!profile.efln && !profile.enderference);
    }
    #[test]
    fn ensure_materials_fills_missing_materials_but_rebuild_alone_does_not() {
        let mut registry = Registry::default();
        registry.reload(&[], &Always);
        let (wood, stone) = (Identifier::parse("test:wood").unwrap(), Identifier::parse("test:stone").unwrap());
        let mut registry_materials = loaded_materials();
        registry_materials.stats = crate::material_stats::load(&[
            pluto_lib::JsonResourceStack { id: wood.clone(), layers: vec![
                Ok(json!({"stats":{"tconstruct:head":{"durability":30},"tconstruct:handle":{"durability":0.5}}})),
            ] },
            pluto_lib::JsonResourceStack { id: stone.clone(), layers: vec![
                Ok(json!({"stats":{"tconstruct:head":{"durability":90},"tconstruct:handle":{"durability":0.5}}})),
            ] },
        ], &mut registry_materials.problems);
        for (id, tier, sort_order) in [(&wood, 1, 1), (&stone, 2, 2)] {
            registry_materials.materials.insert(id.clone(), crate::materials::Material {
                id: id.clone(), tier, sort_order, rarity: crate::materials::Rarity::Common, craftable: true, hidden: false,
            });
        }
        registry_materials.visible = vec![wood.clone(), stone.clone()];
        let data = Data::decode(&json!({"modules":[
            {"type":"tconstruct:material_stats","stat_types":["tconstruct:head","tconstruct:handle"]},
            {"type":"tconstruct:default_materials","materials":[
                {"type":"tconstruct:fixed","material":"test:stone"},
                {"type":"tconstruct:fixed","material":"test:wood"}
            ]}
        ]})).unwrap();
        let env = Environment { definition: Some(&data), has_materials: true, materials: &registry_materials, modifiers: &registry, item_tags_loaded: true, item: None, item_tags: None };
        let names = |nbt: &NbtCompound| materials(nbt, true).as_slice().iter().map(ToString::to_string).collect::<Vec<_>>();
        // rebuild_stats alone never fills: a station-built stack keeps the materials it was
        // built with (the original fills on inventory tick, in ensureHasData).
        let mut nbt = NbtCompound::new();
        assert!(rebuild_stats(&mut nbt, &env).unwrap());
        assert!(names(&nbt).is_empty());
        // ensure_materials (the verifyTag path) fills a bare stack and writes the fill back.
        let mut nbt = NbtCompound::new();
        assert!(ensure_materials(&mut nbt, &env));
        assert_eq!(names(&nbt), vec!["test:stone", "test:wood"]);
        // A partial list keeps its material and fills only the missing position.
        let mut nbt = NbtCompound::new();
        set_materials_raw(&mut nbt, &MaterialList::from_variants(vec![crate::tool_materials::MaterialVariant::parse("test:wood").unwrap()]));
        assert!(ensure_materials(&mut nbt, &env));
        assert_eq!(names(&nbt), vec!["test:wood", "test:wood"]);
        // A full list is left alone.
        assert!(!ensure_materials(&mut nbt, &env));
    }
    #[test]
    fn mining_component_follows_stored_speed_tier_and_broken_state() {
        let mut registry = Registry::default();
        registry.reload(&[], &Always);
        let materials = loaded_materials();
        let data = Data::decode(&json!({"modules":[
            {"type":"tconstruct:base_stats","stats":{"tconstruct:durability":10,"tconstruct:mining_speed":6,"tconstruct:harvest_tier":"minecraft:iron"}},
            {"type":"tconstruct:is_effective","predicate_type":"mantle:tag","tag":"minecraft:mineable/pickaxe"}
        ]})).unwrap();
        let mut nbt = NbtCompound::new();
        assert!(rebuild_stats(&mut nbt, &environment(&data, &materials, &registry)).unwrap());
        assert_eq!(stat(&nbt, Stat::MiningSpeed), 6.0);
        let tool = mining_component(&nbt, &data).unwrap().unwrap();
        assert_eq!(tool["default_mining_speed"], Nbt::Float(1.0));
        let Nbt::List(rules) = &tool["rules"] else { panic!() };
        assert_eq!(rules.len(), 2, "iron tier rule plus the effective rule");
        // Breaking the tool flips it to the slow, drop-less component.
        set_damage(&mut nbt, 10);
        let broken = mining_component(&nbt, &data).unwrap().unwrap();
        assert_eq!(broken["default_mining_speed"], Nbt::Float(0.3));
        // A definition with no mining module has none.
        assert!(mining_component(&nbt, &Data::default()).unwrap().is_none());
    }
    #[test]
    fn validation_rejects_overspent_slots_and_unmet_modifier_requirements() {
        let mut registry = Registry::default();
        registry.reload(&[
            (Identifier::parse("test:needs").unwrap(), json!({"modules":[{"type":"tconstruct:requirements","translation_key":"test.needs.base",
                "requirement":{"type":"tconstruct:has_modifier","check":"all","modifier":{"type":"tconstruct:single","modifier":"test:base"}}}]})),
            (Identifier::parse("test:base").unwrap(), json!({})),
        ], &Always);
        let materials = loaded_materials();
        let data = Data::default();
        let env = environment(&data, &materials, &registry);
        let mut nbt = NbtCompound::new();
        assert_eq!(try_validate(&nbt, &env), None, "an empty tool is valid");
        // Free slots are persistent + volatile; a negative total fails, naming the slot.
        let slots = |name: &str, value: i32| { let mut data = NbtCompound::new(); data.insert(name.into(), Nbt::Int(value)); Nbt::Compound(data) };
        nbt.insert(TAG_PERSISTENT.into(), slots("upgrades", -1));
        assert_eq!(try_validate(&nbt, &env).as_deref(), Some("recipe.tconstruct.modifier.validate_slots slot_type.tconstruct.upgrades"));
        nbt.insert(TAG_VOLATILE.into(), slots("upgrades", 1));
        assert_eq!(try_validate(&nbt, &env), None, "volatile slots pay for the persistent debt");
        // A recipe-added modifier whose requirement fails reports its own translation key.
        let needs = List::default().with_modifier(Identifier::parse("test:needs").unwrap(), 1).unwrap();
        nbt.insert(TAG_UPGRADES.into(), needs.to_nbt());
        nbt.insert(TAG_MODIFIERS.into(), needs.to_nbt());
        assert_eq!(try_validate(&nbt, &env).as_deref(), Some("test.needs.base"));
        let both = needs.with_modifier(Identifier::parse("test:base").unwrap(), 1).unwrap();
        nbt.insert(TAG_MODIFIERS.into(), both.to_nbt());
        assert_eq!(try_validate(&nbt, &env), None, "the requirement reads the tool's full modifier list");
    }
    #[test]
    fn unready_environment_changes_nothing() {
        let registry = Registry::default();
        let materials = loaded_materials();
        let data = Data::default();
        let mut nbt = NbtCompound::new();
        nbt.insert(TAG_VOLATILE.into(), Nbt::Compound(NbtCompound::new()));
        let mut env = environment(&data, &materials, &registry);
        env.item_tags_loaded = false;
        assert!(!rebuild_stats(&mut nbt, &env).unwrap());
        assert!(!is_initialized(&nbt) && nbt.contains_key(TAG_VOLATILE));
        // A fresh registry has not loaded dynamic modifiers, which is also a gate.
        env.item_tags_loaded = true;
        assert!(!rebuild_stats(&mut nbt, &env).unwrap());
    }
}
