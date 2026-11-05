// Retained armor commits at a macro-free native caller, not inside hurt_armor.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;

type Losses = Vec<(ItemStack, u32)>;

fn supported(game: &super::Game<'_>, stack: &ItemStack) -> Option<usize> {
    if game.remote.is_some() || game.creative || stack.count != 1 || stack.wire_patch().is_some() {
        return None;
    }
    let slot = super::tconstruct_fire_duration::armor_slot(&stack.item)?;
    if game.armor[slot].as_ref() != Some(stack) { return None; }
    let item = game.res.items().get(&stack.item)?;
    // The native loop alone owns fixed-maximum gear; never apply it twice.
    if item.max_damage > 0 || !item.equip_damage_on_hurt { return None; }
    if !super::tconstruct_source_armor_wear::valid_record(stack) { return None; }
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    for tag in ["tconstruct:modifiable/armor", "tconstruct:modifiable/durability"] {
        if !tags.get(tag)?.contains(stack.item.as_ref()) { return None; }
    }
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    let mut active = traits.levels;
    let custom = stack.data()?.other("minecraft:custom_data")?;
    match custom.get("tic_upgrades") {
        None => (),
        Some(Tag::Compound(upgrades)) => {
            for (id, value) in upgrades {
                let Tag::Int(level) = value else { return None; };
                if *level < 0 { return None; }
                let total = active.entry(id.to_string_lossy().to_string()).or_default();
                *total = total.checked_add(*level)?;
            }
        }
        _ => return None,
    }
    let raw = game.world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()?.as_ref().ok()?;
    for (id, level) in active {
        if level < 0 { return None; }
        if level == 0 { continue; }
        let (namespace, path) = id.split_once(':')?;
        let value = serde_json::from_slice::<serde_json::Value>(raw.0.get(&format!("{namespace}:tinkering/modifiers/{path}"))?.last()?).ok()?;
        // Only exact reviewed definitions with no tool-damage hook. Unknown,
        // changed and reduction-bearing modifiers leave this piece untouched.
        if !super::tconstruct_armor_wear_definitions::supported(&id, &value) { return None; }
    }
    Some(slot)
}

pub(super) fn record(game: &mut super::Game<'_>, stack: &ItemStack, loss: u32) {
    let actor = game.lan_acting;
    if !game.world.chunks.__horizon_mixins_tconstruct_armor_wear_context().contains_key(&actor)
        || supported(game, stack).is_none() { return; }
    if let Some(pending) = game.world.chunks.__horizon_mixins_tconstruct_armor_wear_context_mut().get_mut(&actor) {
        pending.push((stack.clone(), loss));
    }
}

fn commit(game: &mut super::Game<'_>, losses: Losses) {
    let selected = super::tconstruct_source_armor_wear::unambiguous_losses(&game.armor, &losses);
    let mut changed = false;
    let mut broke = false;
    for (slot, loss) in selected {
        let Some(original) = game.armor[slot].clone() else { continue; };
        if supported(game, &original) != Some(slot) { continue; }
        let stack = game.armor[slot].as_mut().expect("matched armor slot");
        super::tconstruct_tx::fixed_damage(stack, loss);
        changed |= *stack != original;
        broke |= super::tconstruct_source_ranged_wear::is_broken(stack);
    }
    if broke { game.apply_equipment(); }
    if changed { super::tconstruct_maintained_world::refresh(game); }
}

pub(super) fn wrap(
    game: &mut super::Game<'_>, source: horizon_world::damage_source::DamageTypeId, damage: f32,
    original: impl FnOnce(&mut super::Game<'_>, horizon_world::damage_source::DamageTypeId, f32),
) {
    if game.remote.is_some() || game.creative || !damage.is_finite() || damage <= 0.0 {
        original(game, source, damage);
        return;
    }
    let actor = game.lan_acting;
    let previous = game.world.chunks.__horizon_mixins_tconstruct_armor_wear_context_mut().insert(actor, Vec::new());
    original(game, source, damage);
    let losses = game.world.chunks.__horizon_mixins_tconstruct_armor_wear_context_mut().remove(&actor).unwrap_or_default();
    // Commit before the next ArmorHurt event. No collector is active during
    // equipment refresh, and an enclosing actor scope is restored afterward.
    if game.lan_acting == actor { commit(game, losses); }
    if let Some(previous) = previous {
        game.world.chunks.__horizon_mixins_tconstruct_armor_wear_context_mut().insert(actor, previous);
    }
}
