use super::{tconstruct_dragonfall as rule, Game};
use horizon_core::position::BlockPos;
use horizon_world::{
    climbing::{self, ClimbingWorld},
    registry::BlockStateId,
};

fn levels(game: &Game<'_>) -> Option<([i32; 4], [i32; 4])> {
    if game.remote.is_some() {
        return None;
    }
    let raw = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()?
        .as_ref()
        .ok()?;
    if !rule::known(super::tconstruct_modifier_snapshot::highest_definition(
        raw,
        "tconstruct:tinkering/modifiers/dragonfall",
    )) {
        return None;
    }
    let tags = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_resolved_item_tags()
        .as_ref()?
        .as_ref()
        .ok()?;
    let worn = tags.get("tconstruct:modifiable/armor/worn")?;
    let (mut critical, mut fall) = ([0; 4], [0; 4]);
    for (index, stack) in game
        .armor
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.as_ref().map(|s| (i, s)))
    {
        if rule::armor_slot(&stack.item) != Some(index) || !worn.contains(stack.item.as_ref()) {
            continue;
        }
        if stack.count != 1 || stack.wire_patch().is_some() {
            return None;
        }
        let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
        let level =
            super::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, rule::ID)?;
        // Critical is percent-style equipment-change; safe-fall is the normal
        // item-attribute hook, which ModifiableArmorItem disables when broken.
        critical[index] = level;
        if super::tconstruct_source_melee_shape::intact(stack).is_some() {
            fall[index] = level;
        }
    }
    Some((critical, fall))
}

/// Read-only source airborne predicate using the native climbing registry.
/// No newer glide-through exception: source 1.20.1 checks ordinary climbing.
struct ClimbProbe<'a, 'w> {
    game: &'a Game<'w>,
    position: BlockPos,
}
impl ClimbingWorld for ClimbProbe<'_, '_> {
    type Error = std::convert::Infallible;
    fn is_spectator(&mut self) -> Result<bool, Self::Error> {
        Ok(self.game.player.as_ref().is_some_and(|p| p.spectator))
    }
    fn block_position(&mut self) -> Result<BlockPos, Self::Error> {
        Ok(self.position)
    }
    fn in_block_state(&mut self) -> Result<BlockStateId, Self::Error> {
        self.block_state(self.position)
    }
    fn is_fall_flying(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn block_state(&mut self, pos: BlockPos) -> Result<BlockStateId, Self::Error> {
        Ok(self.game.world.block(pos.x, pos.y, pos.z))
    }
    fn set_last_climbable_position(&mut self, _: BlockPos) {}
}
fn airborne(game: &Game<'_>) -> bool {
    let Some(player) = game.player.as_ref() else {
        return false;
    };
    if player.ground.on_ground || player.fluids.was_touching_water || game.riding.is_some() {
        return false;
    }
    if !player.position.x.is_finite()
        || !player.position.y.is_finite()
        || !player.position.z.is_finite()
    {
        return false;
    }
    // Player.onClimbable is false while the flight ability is active.
    if player.flying {
        return true;
    }
    let mut probe = ClimbProbe {
        game,
        position: BlockPos::containing(player.position),
    };
    climbing::on_climbable(&mut probe, &game.motion.climbing).is_ok_and(|climbing| !climbing)
}

pub(super) fn critical(game: &Game<'_>, critical: &mut bool, multiplier: &mut f32) {
    // Existing source secondary-hit handling clears only the ordinary critical
    // contribution. Never introduce a Dragonfall crit into that AOE context.
    if game
        .world
        .chunks
        .__horizon_mixins_tconstruct_melee_active()
        .contains(&game.lan_acting)
    {
        return;
    }
    let Some((levels, _)) = levels(game) else {
        return;
    };
    if let Some((is_critical, value)) =
        rule::critical(*multiplier, *critical, airborne(game), &levels)
    {
        *critical = is_critical;
        *multiplier = value;
    }
}

pub(super) fn refresh(game: &mut Game<'_>) {
    if game.remote.is_some() {
        return;
    }
    let bonus = levels(game)
        .and_then(|(_, levels)| rule::safe_fall_bonus(&levels))
        .unwrap_or(0.0);
    let Some(player) = game.player.as_mut() else {
        return;
    };
    // Native safe-fall distance is the total threshold (base three), whereas
    // TiC's attribute is a distance reduction. Add the bounded TiC contribution.
    let _ = player.set_modifier(
        "minecraft:safe_fall_distance",
        "tconstruct:dragonfall_worn",
        bonus,
        "add_value",
        bonus > 0.0,
    );
}
