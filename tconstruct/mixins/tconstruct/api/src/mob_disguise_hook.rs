//! Connects the mob disguise modifiers to PlutoLib. A mob that works out how far it sees a wearer asks the visibility event, which scales
//! the range by the worn pieces' disguises for that mob type (`ToolEvents.livingVisibility`); the Enderman's stare asks the ender-mask
//! query, which a helmet with the `tconstruct:endermask` volatile flag answers yes (`ModifiableArmorItem.isEnderMask`).
use crate::{break_speed_hook::with_held_tool, mob_disguise, tool_stack};
use pluto_lib::{EnderMaskContext, Identifier, ItemBehavior, ItemBehaviors, ServerLivingEntityEvents, StackData};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The volatile flag that makes a helmet an ender mask (`ModifiableArmorItem.ENDERMASK`).
pub(crate) const ENDERMASK: &str = "tconstruct:endermask";

/// How long a worked-out count is trusted while the armor stays the same: a reload of the modifier data does not change the armor.
const COUNT_LIFETIME: Duration = Duration::from_secs(5);

/// The wearer's count of disguises for each mob type, from the pieces they wear; `None` when a Tinkers piece could not be read (the
/// registries are not loaded yet), which is not worth remembering.
fn counts_of(armor: &[(String, StackData)]) -> Option<BTreeMap<String, i32>> {
    let mut found = Vec::new();
    for (_, piece) in armor {
        match with_held_tool(piece, |env, stack| tool_stack::mob_disguises(&stack.custom_data, env)) {
            Ok(disguises) => found.extend(disguises),
            // Armor that is not a Tinkers tool, or a broken piece, adds nothing.
            Err(reason) if reason == "held stack is not a Tinkers tool" || reason == "the tool is broken" => {}
            Err(_) => return None,
        }
    }
    Some(mob_disguise::count(found))
}

/// The pieces' disguises, kept for each wearer until their armor changes: a mob asks for every player it can see on every tick.
fn cached_counts(wearer: u128, revision: u64, armor: &[(String, StackData)]) -> Arc<BTreeMap<String, i32>> {
    static CACHE: Mutex<Option<HashMap<u128, (u64, Instant, Arc<BTreeMap<String, i32>>)>>> = Mutex::new(None);
    if let Ok(cache) = CACHE.lock() {
        if let Some((known, at, counts)) = cache.as_ref().and_then(|cache| cache.get(&wearer)) {
            if *known == revision && at.elapsed() < COUNT_LIFETIME { return counts.clone(); }
        }
    }
    let Some(counts) = counts_of(armor) else { return Arc::new(BTreeMap::new()) };
    let counts = Arc::new(counts);
    if let Ok(mut cache) = CACHE.lock() { cache.get_or_insert_with(HashMap::new).insert(wearer, (revision, Instant::now(), counts.clone())); }
    counts
}

/// Whether a worn helmet is an ender mask: its volatile data carries the flag. Without the worn stack nothing is claimed.
fn is_ender_mask(query: &EnderMaskContext) -> Option<bool> {
    let stack = query.stack.as_ref()?;
    tool_stack::volatile_flag(&stack.custom_data, ENDERMASK).then_some(true)
}

/// Registers the callbacks. Called once from the product's hooks.
pub(crate) fn register() {
    ServerLivingEntityEvents::MODIFY_VISIBILITY.register(|look| {
        let counts = cached_counts(look.wearer, look.armor_revision, &look.armor);
        let factor = mob_disguise::visibility_factor(&counts, &look.observer);
        if factor != 1.0 { look.multiplier.set(look.multiplier.get() * factor); }
    });
    // The Enderman's stare asks about the head slot only, and the helmets are the only Tinkers armor that can sit there.
    for family in ["travelers", "plate", "slime"] {
        let Ok(item) = Identifier::new("tconstruct", &format!("{family}_helmet")) else { continue };
        if let Err(error) = ItemBehaviors::register(ItemBehavior::new(item).is_ender_mask(is_ender_mask)) {
            eprintln!("Tinkers: {family} helmet ender mask not registered: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{Nbt, NbtCompound};

    fn helmet(volatile: Option<(&str, i8)>) -> StackData {
        let mut stack = StackData { item: "tconstruct:travelers_helmet".into(), count: 1, ..StackData::default() };
        if let Some((flag, value)) = volatile {
            let mut data = NbtCompound::new();
            data.insert(flag.into(), Nbt::Byte(value));
            stack.custom_data.insert(tool_stack::TAG_VOLATILE.into(), Nbt::Compound(data));
        }
        stack
    }
    fn query(stack: Option<StackData>) -> EnderMaskContext {
        EnderMaskContext { item: "tconstruct:travelers_helmet".into(), stack, wearer_uuid: Some(1), observer_entity: Some(2) }
    }

    #[test]
    fn only_a_helmet_carrying_the_endermask_flag_is_a_mask() {
        assert_eq!(is_ender_mask(&query(Some(helmet(Some((ENDERMASK, 1)))))), Some(true));
        assert_eq!(is_ender_mask(&query(Some(helmet(Some((ENDERMASK, 0)))))), None, "a cleared flag leaves the native answer");
        assert_eq!(is_ender_mask(&query(Some(helmet(Some(("tconstruct:other", 1)))))), None);
        assert_eq!(is_ender_mask(&query(Some(helmet(None)))), None);
        assert_eq!(is_ender_mask(&query(None)), None, "no worn stack, no claim");
    }
}
