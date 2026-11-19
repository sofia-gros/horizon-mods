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
    Ok(true)
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
    with_tool_facts(nbt, env, |saved, facts| {
        let mut modules: Vec<(i32, usize, &crate::modifier_entries::Entry, &crate::modifier_modules::Module)> = Vec::new();
        for (index, (entry, modifier)) in saved.iter().enumerate() {
            for (priority, module) in modifier.mining_modules() { modules.push((priority, index, entry, module)); }
        }
        modules.sort_by_key(|(priority, index, _, _)| (std::cmp::Reverse(*priority), *index));
        let view = crate::mining_speed::ToolView::of(nbt);
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

/// The tool actions the tool's modifiers perform on a right click, in modifier order.
pub(crate) fn tool_actions(nbt: &NbtCompound, env: &Environment) -> Vec<(crate::tool_actions::ToolAction, String, bool)> {
    with_tool_facts(nbt, env, |saved, facts| saved.iter().flat_map(|(entry, modifier)| modifier.tool_actions(entry, facts)).collect())
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

/// ModifyDamageModifierHook over every modifier of the tool, in modifier order, starting from `amount`: the damage its wearer takes.
pub(crate) fn adjust_damage(nbt: &NbtCompound, env: &Environment, source: &crate::protection::DamageSource, entities: &crate::entity_facts::Entities, amount: f32) -> f32 {
    with_tool_facts(nbt, env, |saved, facts| saved.iter().fold(amount, |amount, (entry, modifier)| if amount > 0.0 { modifier.adjust_damage(entry, facts, nbt, source, entities, amount) } else { amount }))
}

/// The attribute modifiers the tool's modifiers add for the `wanted` slots (AttributesModifierHook), in modifier order.
pub(crate) fn attribute_entries(nbt: &NbtCompound, env: &Environment, wanted: &[&str]) -> Vec<NbtCompound> {
    with_tool_facts(nbt, env, |saved, facts| saved.iter().flat_map(|(entry, modifier)| modifier.attribute_entries(entry, facts, nbt, wanted)).collect())
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

/// The claims a worn armor piece makes on attributes through max armor attribute modules, in modifier order.
pub(crate) fn max_armor_claims(nbt: &NbtCompound, env: &Environment) -> Vec<crate::max_armor::Candidate> {
    let broken = is_broken(nbt);
    with_tool_facts(nbt, env, |saved, facts| {
        saved.iter().flat_map(|(entry, modifier)| modifier.max_armor_attribute_modules().filter_map(move |module| module.max_armor_candidate(entry, facts, broken)).collect::<Vec<_>>()).collect()
    })
}

/// The levels a worn armor piece adds under each armor level key, in modifier order.
pub(crate) fn armor_levels(nbt: &NbtCompound, env: &Environment) -> Vec<(Identifier, i32)> {
    let broken = is_broken(nbt);
    saved_modifiers(nbt, env.modifiers).iter().flat_map(|(entry, modifier)| modifier.armor_level_modules().filter_map(move |module| module.armor_level(entry, broken)).collect::<Vec<_>>()).collect()
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

/// The mob effects a worn armor piece puts on the mob that hit its wearer, in modifier order. `in_tag` answers the entity type tags and the
/// pseudo state tags of the attacking mob; `rolls` gives uniform values in [0, 1).
pub(crate) fn counter_effects(nbt: &NbtCompound, env: &Environment, in_tag: &dyn Fn(&Identifier) -> bool, rolls: &mut dyn FnMut() -> f32) -> Vec<(crate::weapon_effect::Whom, crate::weapon_effect::Grant)> {
    with_tool_facts(nbt, env, |saved, facts| {
        let mut grants = Vec::new();
        for (entry, modifier) in saved {
            for module in modifier.counter_effect_modules() {
                if let Some(grant) = module.counter_effect(entry, facts, in_tag, rolls) { grants.push(grant); }
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
    with_tool_facts(nbt, env, |saved, facts| {
        crate::tool_inventory::ToolInventory::new(saved.iter().rev().flat_map(|(entry, modifier)| modifier.inventory_parts(entry, facts)).collect())
    })
}

/// The slots the tool's volatile data counts (ToolInventoryCapability.TOTAL_SLOTS); the capability exists only above zero.
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
            amount = modifier.reduce_damage(entry, facts, cause, &modifier_in_tag, amount, random);
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
