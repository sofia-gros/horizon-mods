use super::{Game, tconstruct_strong_bones as rules};
use horizon_client::effects::EffectInstance;

fn helmet(game: &Game<'_>) -> Option<(String, i32)> {
    if game.remote.is_some() {
        return None;
    }
    let stack = game.armor[0].as_ref()?;
    if !matches!(
        stack.item.as_ref(),
        "tconstruct:travelers_helmet" | "tconstruct:plate_helmet" | "tconstruct:slime_helmet"
    ) || stack.count != 1
        || stack.wire_patch().is_some()
    {
        return None;
    }
    let tags = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_resolved_item_tags()
        .as_ref()?
        .as_ref()
        .ok()?;
    if !tags
        .get("tconstruct:modifiable")?
        .contains(stack.item.as_ref())
    {
        return None;
    }
    let raw = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()?
        .as_ref()
        .ok()?;
    // Strong Bones is a registered static Java modifier, not a JSON module.
    if raw
        .0
        .contains_key("tconstruct:tinkering/modifiers/strong_bones")
    {
        return None;
    }
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    let level =
        super::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, rules::ID)?;
    // ModifierUtil.getModifierLevel and the equipment cure hook do not require
    // an intact helmet. A broken Bone Skull can still receive the effect.
    (level > 0).then(|| (stack.item.to_string(), level))
}

pub(super) fn finish(game: &mut Game<'_>, consumed: &horizon_items::items::ItemStack) {
    if consumed.item.as_ref() != "minecraft:milk_bucket" || game.player.is_none() {
        return;
    }
    let Some((item, level)) = helmet(game) else {
        return;
    };
    let Some(duration) = rules::milk_duration(level) else {
        return;
    };
    // Source Finish runs after milk curing. Use normal add/update semantics,
    // including stronger or hidden Resistance, rather than replacing the list.
    let mut effect = EffectInstance::new("resistance", duration, 0);
    *effect.__horizon_mixins_tconstruct_strong_bones_cure_item_mut() = Some(item.clone());
    let actor = game.lan_acting;
    game.world
        .chunks
        .__horizon_mixins_tconstruct_strong_bones_helmets_mut()
        .insert(actor, Some(item));
    game.add_player_effect(effect);
}

pub(super) fn equipment(game: &mut Game<'_>) {
    if game.remote.is_some() {
        return;
    }
    let actor = game.lan_acting;
    let current = helmet(game).map(|(item, _)| item);
    let previous = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_strong_bones_helmets()
        .get(&actor)
        .cloned()
        .unwrap_or_else(|| {
            // Recover the current instance's item when crossing dimensions;
            // the effect travels with the actor, not the world-local tracker.
            game.potions.effects.get("resistance").and_then(|effect| {
                effect
                    .__horizon_mixins_tconstruct_strong_bones_cure_item()
                    .clone()
            })
        });
    game.world
        .chunks
        .__horizon_mixins_tconstruct_strong_bones_helmets_mut()
        .insert(actor, current.clone());
    let Some(previous) = previous.filter(|previous| current.as_ref() != Some(previous)) else {
        return;
    };
    // Source curePotionEffects uses the removed helmet's item type, and only
    // the active instance's curatives. Do not delete an unrelated Resistance.
    let before = game.potions.effects.list.len();
    game.potions.effects.list.retain(|effect| {
        effect
            .__horizon_mixins_tconstruct_strong_bones_cure_item()
            .as_deref()
            != Some(previous.as_str())
    });
    if game.potions.effects.list.len() != before {
        game.apply_effect_status();
    }
}
