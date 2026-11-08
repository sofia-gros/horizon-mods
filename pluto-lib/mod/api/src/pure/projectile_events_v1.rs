//! Projectiles hitting things (Forge: `ProjectileImpactEvent`). A callback sees what flew, who threw it and what it met, can act on the
//! world with [`World`] and can cancel the hit.

use super::base::Event;
use super::block_entity_api_v1::BlockPos;
use super::world_api_v1::World;

/// A projectile that has found what it meets.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectileImpact {
    /// The dimension, such as `minecraft:overworld`; also available as [`ProjectileImpact::world`].
    pub dimension: String,
    /// The projectile's entity id.
    pub projectile: u32,
    /// The projectile's type, such as `minecraft:snowball` or a mod's thrown type.
    pub kind: String,
    /// The entity that threw it (negative if none, such as a dispenser).
    pub owner: i64,
    /// The entity it hit, if it hit one.
    pub entity: Option<u32>,
    /// The type of the entity it hit; empty if it hit a block.
    pub entity_kind: String,
    /// The block it hit.
    pub block: BlockPos,
    /// The face it hit as a direction vector, `[0, 1, 0]` for the top.
    pub face: [i32; 3],
    /// Where it hit.
    pub at: [f64; 3],
}

impl ProjectileImpact {
    /// The world the projectile is in.
    pub fn world(&self) -> World {
        World::new(&self.dimension)
    }
}

static IMPACT: Event<ProjectileImpact, bool> = Event::allow_all();

/// Events about projectiles.
pub struct ProjectileEvents;

impl ProjectileEvents {
    /// A projectile hit something, before the hit acts. Answer `false` to cancel the hit (the projectile flies on); one `false` is enough.
    /// Only projectiles the game flies (arrows, tridents, snowballs and mod thrown types, eggs, pearls, potions, fireballs, ...) are heard.
    pub const IMPACT: &'static Event<ProjectileImpact, bool> = &IMPACT;
}

/// Whether anyone listens, so the game only keeps the copies it needs when someone does.
pub(crate) fn heard() -> bool {
    !IMPACT.is_empty()
}

pub(crate) fn impact(impact: &ProjectileImpact) -> bool {
    IMPACT.invoke(impact)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pure::base::Identifier;
    use crate::pure::world_api_v1::{with_world, BlockState, MemoryWorldOps};

    #[test]
    fn a_listener_acts_on_the_world_where_the_projectile_landed_and_can_cancel_the_hit() {
        assert!(!heard());
        ProjectileEvents::IMPACT.register(|hit| {
            if hit.kind == "example:light_ball" {
                hit.world().set_block_state(hit.block, &BlockState::new(Identifier::parse("minecraft:light").unwrap()));
                false
            } else {
                true
            }
        });
        assert!(heard());
        let hit = |kind: &str| ProjectileImpact {
            dimension: "minecraft:overworld".into(),
            projectile: 7,
            kind: kind.into(),
            owner: 1,
            entity: None,
            entity_kind: String::new(),
            block: [4, 70, 4],
            face: [0, 1, 0],
            at: [4.5, 71.0, 4.5],
        };
        let mut ops = MemoryWorldOps::default();
        with_world(&mut ops, || {
            assert!(!impact(&hit("example:light_ball")), "the ball's hit is cancelled");
            assert!(impact(&hit("minecraft:snowball")), "other projectiles hit as usual");
        });
        assert_eq!(ops.blocks.len(), 1);
    }
}
