use super::{Game, tconstruct_shield as rule};
use horizon_client::items::ItemStack;
use horizon_world::nbt::Tag;

pub(super) fn valid_stack(stack: &ItemStack) -> bool {
    if !rule::stock(&stack.item)
        || stack.count != 1
        || !super::tconstruct_source_tool_normalize::patch_safe_for_tool_mutation(stack)
        || super::tconstruct_source_melee_shape::intact(stack).is_none()
    {
        return false;
    }
    let Some(data) = stack.data() else {
        return false;
    };
    if data.other("minecraft:blocks_attacks").is_some() {
        return false;
    }
    matches!(
        data.other("minecraft:custom_data")
            .and_then(|c| c.get("tic_broken")),
        None | Some(Tag::Byte(0))
    )
}
pub(super) fn stats(game: &Game<'_>, stack: &ItemStack) -> Option<rule::Stats> {
    if game.remote.is_some() || !valid_stack(stack) {
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
    let definition = |id| super::tconstruct_modifier_snapshot::highest_definition(raw, id);
    if !rule::known_blocking(definition("tconstruct:tinkering/modifiers/blocking")) {
        return None;
    }
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    if super::tconstruct_source_material_traits::level_with_upgrade(
        stack,
        &traits,
        "tconstruct:blocking",
    )? <= 0
    {
        return None;
    }
    let blockade = super::tconstruct_source_material_traits::level_with_upgrade(
        stack,
        &traits,
        "tconstruct:blockade",
    )?;
    if blockade > 0 && !rule::known_blockade(definition("tconstruct:tinkering/modifiers/blockade"))
    {
        return None;
    }
    let materials = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_material_snapshot()
        .as_ref()?;
    let name = stack.item.strip_prefix("tconstruct:")?;
    rule::stats(
        materials
            .get(&format!("tconstruct:tinkering/tool_definitions/{name}"))?
            .last()?,
        blockade,
    )
}
pub(super) fn blocking(
    game: &Game<'_>,
    from: Option<[f64; 3]>,
    damage_type: &str,
    pierce: bool,
) -> Option<horizon_client::health::Blocking> {
    let (slot, ticks) = game.using?;
    if ticks < 5 {
        return None;
    }
    let stats = stats(game, game.stack_in(slot)?)?;
    let p = game.player.as_ref()?;
    if !rule::faces(
        [p.position.x, p.position.y, p.position.z],
        from,
        game.yaw,
        game.pitch,
        stats.angle,
    ) {
        return None;
    }
    Some(horizon_client::health::Blocking {
        rules: horizon_world::blocking_rules::BlockingRules {
            delay_seconds: 0.25,
            disable_scale: 1.0,
            reductions: vec![horizon_world::blocking_rules::DamageReduction {
                horizontal_angle: 90.0,
                types: None,
                base: stats.amount,
                factor: 0.0,
            }],
            item_damage: horizon_world::blocking_rules::ItemDamageFunction {
                threshold: 3.0,
                base: 0.0,
                factor: 0.0,
            },
        },
        bypassed: game
            .stations()
            .enchantments
            .damage_tags
            .get("minecraft:bypasses_shield")
            .is_some_and(|tags| tags.contains(damage_type)),
        pierce,
        angle: 0.0, // Source cone was checked above, including pitch.
    })
}
pub(super) fn wear(game: &mut Game<'_>, outcome: horizon_client::health::BlockOutcome) {
    if game.remote.is_some() || outcome.blocked <= 0.0 || !outcome.blocked.is_finite() {
        return;
    }
    let Some((slot, _)) = game.using else {
        return;
    };
    let Some(stack) = game.stack_in(slot).cloned() else {
        return;
    };
    if stats(game, &stack).is_none() {
        return;
    }
    game.award_stat("used", &stack.item, 1);
    let amount = rule::wear(outcome.blocked);
    if game.creative || amount == 0 {
        return;
    }
    let Some(mut updated) =
        super::tconstruct_source_tool_normalize::clone_for_tool_mutation(&stack)
    else {
        return;
    };
    let (loss, spent) = super::tconstruct_source_overslime_host::damage(game, &stack, amount);
    super::tconstruct_source_overslime::spend(&mut updated, spent);
    super::tconstruct_tx::fixed_damage(&mut updated, loss);
    let broken = super::tconstruct_source_ranged_wear::is_broken(&updated);
    if game.stack_in(slot) != Some(&stack) {
        return;
    }
    *game.stack_in_mut(slot) = Some(updated);
    if broken {
        // Retain the broken tool; native static-item durability must not delete it.
        use horizon_core::random::RandomSource;
        let at = game
            .player
            .as_ref()
            .map(|p| [p.position.x, p.position.y, p.position.z]);
        let pitch = 0.8 + game.world.random.next_float() * 0.4;
        if outcome.sound {
            game.audio
                .play("minecraft:item.shield.block", at, 1.0, pitch);
        }
        game.audio
            .play("minecraft:item.shield.break", at, 0.8, pitch);
        game.using = None;
        game.apply_equipment();
    }
}
