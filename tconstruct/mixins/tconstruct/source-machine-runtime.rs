// Authoritative machine state uses normal live inventories plus custom NBT
// fields preserved by the source-machine save hooks.
use horizon_client::containers::BlockEntity;
use horizon_world::nbt::Tag;
use super::{tconstruct_source_melter_state::{Casting, Faucet, Melter}, tconstruct_source_melter_store as store};

type Pos = (i32, i32, i32);

pub(super) fn kind(block: &str) -> Option<&'static str> {
    match block {
        "tconstruct:seared_melter" => Some("tconstruct:melter"),
        "tconstruct:seared_heater" => Some("tconstruct:heater"),
        "tconstruct:seared_faucet" => Some("tconstruct:faucet"),
        "tconstruct:seared_table" => Some("tconstruct:table"),
        "tconstruct:seared_basin" => Some("tconstruct:basin"),
        _ => None,
    }
}

fn initial(kind: &str) -> Option<Tag> {
    Some(match kind {
        "tconstruct:melter" => Melter::new().to_nbt(),
        "tconstruct:heater" => Tag::compound(Vec::<(&str, Tag)>::new()),
        "tconstruct:faucet" => Faucet::new().to_nbt(),
        "tconstruct:table" | "tconstruct:basin" => Casting::new().to_nbt(),
        _ => return None,
    })
}

pub(super) fn ensure(game: &mut super::Game<'_>, pos: Pos, block: &str) -> bool {
    let Some(kind) = kind(block) else { return false; };
    if !game.world.chunks.contains_key(&(pos.0 >> 4, pos.2 >> 4)) { return false; }
    if game.world.block_entities.get(&pos).is_some_and(|entity| !matches!(entity, BlockEntity::Chest(_))) { return false; }
    if store::read(&game.world, pos, kind).is_none() {
        let Some(data) = initial(kind) else { return false; };
        if !store::write(&mut game.world, pos, kind, data) { return false; }
    }
    game.world.block_entities.get_or_insert_with(pos, || BlockEntity::Chest(vec![None; 27]));
    game.world.modified.insert((pos.0 >> 4, pos.2 >> 4));
    true
}

fn same_material(a: &Option<horizon_items::items::ItemStack>, b: &Option<horizon_items::items::ItemStack>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.item == b.item && a.damage == b.damage && a.extra == b.extra,
        _ => false,
    }
}

fn solid_fuel_rule(game: &super::Game<'_>) -> Option<(u32, u32)> {
    let recipes = game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref()?;
    let bytes = recipes.get("tconstruct:smeltery/melting/fuel/solid")?;
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    if value.get("type")?.as_str()? != "tconstruct:melting_fuel" { return None; }
    Some((u32::try_from(value.get("temperature")?.as_u64()?).ok()?, u32::try_from(value.get("rate")?.as_u64()?).ok()?))
}

fn lit(game: &mut super::Game<'_>, pos: Pos, active: bool) {
    let id = game.world.block(pos.0, pos.1, pos.2);
    let Some(mut state) = game.pack.assets.registry.state(id).cloned() else { return; };
    let Some(previous) = state.properties.get_mut("lit") else { return; };
    if previous == if active { "true" } else { "false" } { return; }
    *previous = active.to_string();
    if let Ok(id) = game.pack.assets.registry.resolve(&state) { let _ = game.world.set(pos.0, pos.1, pos.2, id); }
}

/// Tick an already loaded source melter. The heater below provides one solid
/// fuel item through Horizon's existing fuel provider; a container remainder
/// is never discarded. Recipes are resolved from the current world snapshot.
pub(super) fn tick_melter(game: &mut super::Game<'_>, pos: Pos) {
    let Some(saved) = store::read(&game.world, pos, "tconstruct:melter") else { return; };
    let Some(mut melter) = Melter::from_nbt(&saved) else { return; };
    let Some(BlockEntity::Chest(live)) = game.world.block_entities.get(&pos) else { return; };
    if live.len() < 3 { return; }
    for (index, slot) in live.iter().take(3).enumerate() {
        if !same_material(&melter.slots[index], slot) { melter.progress[index] = 0; }
        melter.slots[index] = slot.clone();
    }
    let below = (pos.0, pos.1 - 1, pos.2);
    let heater_state = game.world.block(below.0, below.1, below.2);
    let has_heater = &*game.res.facts().get(heater_state).name == "tconstruct:seared_heater";
    let recipes: Vec<_> = melter.slots.iter().map(|slot| slot.as_ref().and_then(|stack|
        super::tconstruct_source_melter_world::melting(&game.world, &stack.item))).collect();
    if melter.phase == 0 && has_heater && melter.fuel == 0 {
        if let Some((temperature, rate)) = solid_fuel_rule(game) {
            if recipes.iter().flatten().any(|(_, recipe, _)| recipe.temperature <= temperature) {
                let fuel = match game.world.block_entities.get(&below) {
                    Some(BlockEntity::Chest(slots)) => slots.first().and_then(Clone::clone), _ => None,
                };
                if let Some(fuel) = fuel {
                    let duration = game.res.providers().burn_time(game.res.items(), &fuel, false).max(0) as u32 / 4;
                    let remainder = game.res.items().get(&fuel.item).and_then(|info| info.crafting_remainder.clone());
                    if duration > 0 && (remainder.is_none() || fuel.count == 1) {
                        if let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&below) {
                            if let Some(stack) = slots.first_mut().and_then(Option::as_mut) {
                                stack.count -= 1;
                                if stack.count == 0 {
                                    slots[0] = remainder.map(|id| horizon_items::items::ItemStack::new(&id, 1));
                                }
                                melter.fuel = duration; melter.temperature = temperature; melter.rate = rate;
                                game.world.modified.insert((below.0 >> 4, below.2 >> 4));
                            }
                        }
                    }
                }
            }
        }
    }
    if melter.phase == 2 {
        let burning = has_heater && melter.fuel > 0;
        for (index, recipe) in recipes.iter().enumerate() {
            if let Some((_, recipe, fluid)) = recipe {
                if burning || melter.progress[index] == -1 {
                    melter.heat_slot(index, recipe.temperature, recipe.time, fluid, recipe.output.amount);
                } else { melter.cool_slot(index); }
            } else { melter.progress[index] = 0; }
        }
        if melter.fuel > 0 { melter.fuel -= 1; }
        lit(game, pos, burning);
        if has_heater { lit(game, below, burning); }
    }
    melter.phase = (melter.phase + 1) % 4;
    if let Some(BlockEntity::Chest(live)) = game.world.block_entities.get_mut(&pos) {
        for (dst, src) in live.iter_mut().take(3).zip(&melter.slots) { *dst = src.clone(); }
    }
    let _ = store::write(&mut game.world, pos, "tconstruct:melter", melter.to_nbt());
}
