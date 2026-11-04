fn tconstruct_melee_secondary_damage(value: (f32, f32, bool, bool), game: &mut crate::game::Game<'_>, living: bool) -> (f32, f32, bool, bool) {
    use horizon_world::nbt::Tag;
    let (mut amount, scale, mut crit, mut magic) = value;
    let spin = game.player.as_ref().map_or(0.0, |player| player.auto_spin_attack_dmg);
    if crit && game.world.chunks.__horizon_mixins_tconstruct_melee_active().contains(&game.lan_acting) {
        // Source withAOETarget retains cooldown but clears critical damage.
        let base = if spin > 0.0 { spin } else { game.attack_damage() };
        amount -= 0.5 * base * (0.2 + scale * scale * 0.8);
        crit = false;
    }
    // The host swaps the acting player into game.player for local and guest turns.
    if living && spin <= 0.0 {
        if let (Some(stack), Some(player)) = (game.held(), game.player.as_ref()) {
            if stack.item.starts_with("tconstruct:") {
                if let (Some(custom), Some(traits)) = (
                    stack.data().and_then(|data| data.other("minecraft:custom_data")),
                    crate::game::tconstruct_source_material_traits::levels(&game.world, stack),
                ) {
                    // Retained tools may carry an explicit broken flag even when
                    // their damage is below the maximum (for example after loading).
                    let intact = matches!(custom.get("tic_max_damage"), Some(Tag::Int(max)) if *max > 0)
                        && !crate::game::tconstruct_source_ranged_wear::is_broken(stack);
                    // Only the pinned module set/order is implemented here. Unknown
                    // or removed definitions must not inherit this fixed effect.
                    let level = (|| {
                        // Source canPerformAttack requires the live MELEE tag.
                        // A saved multiplier is not evidence of current eligibility.
                        let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
                        if !tags.get("tconstruct:modifiable/melee")?.contains(&*stack.item) { return None; }
                        let raw = game.world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()?.as_ref().ok()?;
                        let source = crate::game::tconstruct_modifier_snapshot::highest_definition(raw, "tconstruct:tinkering/modifiers/raging");
                        if !crate::game::tconstruct_modifier_snapshot::known_raging_melee(source) { return None; }
                        crate::game::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, "tconstruct:raging")
                    })().unwrap_or(0);
                    if let Some(Tag::Float(bits)) = custom.get("tic_raging_melee_multiplier") {
                        let multiplier = f32::from_bits(*bits);
                        if intact && level > 0 && multiplier.is_finite() && player.health.health().is_finite() {
                            let factor = ((11.0 - player.health.health()) / 10.0).clamp(0.0, 1.0);
                            let flat = factor * level as f32 * 2.5 * multiplier;
                            // Source flat Raging (priority 100) precedes the
                            // percent Jagged and Stonebound traits (priority 75).
                            let jagged = traits.levels.get("tconstruct:jagged").copied().unwrap_or(0);
                            let stonebound = traits.levels.get("tconstruct:stonebound").copied().unwrap_or(0);
                            let adjusted = flat + crate::game::tconstruct_source_material_effects::wear_damage_bonus(
                                flat, stack.damage, traits.durability_multiplier, jagged, stonebound,
                            );
                            amount += adjusted * (0.2 + scale * scale * 0.8);
                            magic |= adjusted > 0.0;
                        }
                    }
                }
            }
        }
    }
    (amount, scale, crit, magic)
}
