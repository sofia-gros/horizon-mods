// UNAPPLIED compilation probe appended only to a separate exact Game overlay.
// These functions are never registered or called by a production hook.
#[allow(dead_code)]
mod pluto_contact_positive_probe {
    use super::{Game, commands::Target, lan::LanHost};
    use horizon_client::{effects::EffectInstance, mod_api::EntityRef, motion::MotionWorld};
    use horizon_core::geometry::Vec3;

    // Uses the real private Game and every real MotionWorld constructor field.
    // Returns only an observation; it does not suspend native contact traversal.
    fn observe_motion(game: &mut Game<'_>) -> Option<Vec3> {
        let player = game.player.as_mut()?;
        let world = MotionWorld {
            entities: &[],
            player,
            scene: &mut game.world,
            assets: &game.pack.assets,
            collision_binding: game.collision,
            resources: game.motion,
            health: &game.health_resources,
        };
        Some(world.player.position)
    }

    // Compilation acceptance would prove sequential reborrowing only. It would
    // NOT establish same-actor identity, continuation validity or authorization.
    fn sequential_borrows(
        game: &mut Game<'_>, effect: EffectInstance,
    ) -> Option<(Vec3, Vec3)> {
        let before = observe_motion(game)?;
        game.add_player_effect(effect);
        let after = observe_motion(game)?;
        Some((before, after))
    }

    // A future real-resource harness must call this through the actual guest
    // owner. No fabricated EntityRef, fallback ID or substitute Game exists.
    // Outer None: guest turn not entered. Inner None: no current player body.
    // Missing handle observations must remain None, never be manufactured or
    // treated as a successful handle-based continuation test.
    fn observe_guest_teleport(
        game: &mut Game<'_>, host: &mut LanHost, conn: u32, to: [f64; 3],
    ) -> Option<Option<(Option<EntityRef>, Option<EntityRef>, Vec3, Option<Vec3>)>> {
        game.as_guest(host, conn, |active| {
            let id = u32::try_from(active.player.as_ref()?.entity_id).ok()?;
            let handle = active.native_entity(id);
            let before = observe_motion(active)?;
            active.teleport_to_in_level(Target::Player, to, None, false);
            let after_handle = active.native_entity(id);
            let after = observe_motion(active);
            Some((handle, after_handle, before, after))
        })
    }
}
