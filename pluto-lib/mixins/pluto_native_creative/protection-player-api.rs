// Player-victim protection service. No equipment-specific policy lives here.
use super::{BlockOutcome, Blocking, HealthResources, HealthRules, PlayerMotion};
use crate::pluto_inventory::{equipment::EquipmentSlots, protection as api};
use horizon_world::{
    combat_rules::pluto_protection::{Context, Policy},
    damage_source::DamageTypeId,
};
/// Activation reads the single host-independent provider registry.
pub fn enabled() -> bool {
    api::enabled()
}
pub(super) fn run(
    player: &mut PlayerMotion,
    resources: &HealthResources,
    rules: HealthRules,
    source: DamageTypeId,
    amount: f32,
    blocking: Option<Blocking>,
    living_cause: bool,
    causing: i32,
    equipment: Option<&crate::pluto_equipment::Snapshot>,
) -> crate::Result<BlockOutcome> {
    let guard = Context::enter(Policy::default());
    let source_view = super::pluto_protection_facts::Source::new(&resources.damage, source);
    let equipment_view = equipment.map(super::pluto_protection_facts::Equipment::capture);
    let request = api::Request {
        victim_entity: player.entity_id,
        captured_dimension: equipment.map(|snapshot| snapshot.dimension()),
        client_side: player.client_side,
        replaying: player.replaying,
        spectator: player.spectator,
        invulnerable: player.invulnerable,
        source: &source_view,
        requested_amount: amount,
        living_non_player_cause: living_cause,
        causing_entity: (causing != 0).then_some(causing),
        equipment: equipment_view
            .as_ref()
            .map(|view| view as &dyn EquipmentSlots),
    };
    let mut policy = Policy::default();
    api::visit(|id, provider| {
        let mut contribution = super::pluto_protection_contribution::Scoped::new(id, &mut policy);
        provider(&request, &mut contribution);
    });
    horizon_world::combat_rules::pluto_protection::set_current(policy);
    guard.allow_reentry();
    // This call consumes the one-shot token before other HEAD hooks and the
    // unchanged body run. Native errors and nested calls retain normal drops.
    player.hurt_caused_by(
        resources,
        rules,
        source,
        amount,
        blocking,
        living_cause,
        causing,
    )
}

/// Explicit low-level native player hit with a fresh equipment snapshot.
/// Prefer the Game adapter: a bare motion cannot prove the caller's world.
/// The caller must provide a snapshot captured from this actor in this world
/// immediately before this call. Never reuse one after a guest/world swap.
/// Equipment is passed directly, not kept in TLS or looked up by entity ID.
/// Nested ordinary hits enter through the wrapper with None; no outer snapshot
/// is inherited. This function does not perform Game's post-hit work.
pub fn hurt_with_equipment(
    player: &mut PlayerMotion,
    resources: &HealthResources,
    rules: HealthRules,
    source: DamageTypeId,
    amount: f32,
    blocking: Option<Blocking>,
    living_cause: bool,
    causing: i32,
    equipment: &crate::pluto_equipment::Snapshot,
) -> crate::Result<BlockOutcome> {
    if player.client_side || player.replaying || equipment.entity() != player.entity_id {
        return Err("equipment damage context requires the same authoritative actor".into());
    }
    if !enabled() {
        return player.hurt_caused_by(
            resources,
            rules,
            source,
            amount,
            blocking,
            living_cause,
            causing,
        );
    }
    run(
        player,
        resources,
        rules,
        source,
        amount,
        blocking,
        living_cause,
        causing,
        Some(equipment),
    )
}

/// Whether this motion is authoritative rather than a client/replay copy.
pub fn equipment_allowed(player: &PlayerMotion) -> bool {
    !player.client_side && !player.replaying
}

/// Adapter dispatch. Absence is explicit and preserves the native path.
pub fn hurt_with_optional_equipment(
    player: &mut PlayerMotion,
    resources: &HealthResources,
    rules: HealthRules,
    source: DamageTypeId,
    amount: f32,
    blocking: Option<Blocking>,
    living_cause: bool,
    causing: i32,
    equipment: Option<&crate::pluto_equipment::Snapshot>,
) -> crate::Result<BlockOutcome> {
    match equipment {
        Some(equipment) => hurt_with_equipment(
            player,
            resources,
            rules,
            source,
            amount,
            blocking,
            living_cause,
            causing,
            equipment,
        ),
        None => player.hurt_caused_by(
            resources,
            rules,
            source,
            amount,
            blocking,
            living_cause,
            causing,
        ),
    }
}
