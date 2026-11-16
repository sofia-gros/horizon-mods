//! Host-independent block path-node classification. A missing answer delegates
//! to the host; an explicit Blocked answer must not be treated as missing.
use super::{BlockPos, BlockState, BlockView};

/// Minecraft's path-node categories, without native state ids or mob storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathNodeType {
    Blocked, Open, Walkable, WalkableDoor, Trapdoor, PowderSnow,
    OnTopOfPowderSnow, Fence, Lava, Water, WaterBorder, Rail,
    UnpassableRail, FireInNeighbor, Fire, DamagingInNeighbor, Damaging,
    DoorOpen, DoorWoodClosed, DoorIronClosed, Breach, Leaves,
    StickyHoney, Cocoa, DamageCautious, OnTopOfTrapdoor, BigMobsCloseToDanger,
}
impl PathNodeType {
    /// The host-independent Minecraft name of this category.
    pub fn name(self) -> &'static str {
        match self {
            Self::Blocked => "BLOCKED", Self::Open => "OPEN", Self::Walkable => "WALKABLE",
            Self::WalkableDoor => "WALKABLE_DOOR", Self::Trapdoor => "TRAPDOOR",
            Self::PowderSnow => "POWDER_SNOW", Self::OnTopOfPowderSnow => "ON_TOP_OF_POWDER_SNOW",
            Self::Fence => "FENCE", Self::Lava => "LAVA", Self::Water => "WATER",
            Self::WaterBorder => "WATER_BORDER", Self::Rail => "RAIL", Self::UnpassableRail => "UNPASSABLE_RAIL",
            Self::FireInNeighbor => "FIRE_IN_NEIGHBOR", Self::Fire => "FIRE",
            Self::DamagingInNeighbor => "DAMAGING_IN_NEIGHBOR", Self::Damaging => "DAMAGING",
            Self::DoorOpen => "DOOR_OPEN", Self::DoorWoodClosed => "DOOR_WOOD_CLOSED",
            Self::DoorIronClosed => "DOOR_IRON_CLOSED", Self::Breach => "BREACH", Self::Leaves => "LEAVES",
            Self::StickyHoney => "STICKY_HONEY", Self::Cocoa => "COCOA", Self::DamageCautious => "DAMAGE_CAUTIOUS",
            Self::OnTopOfTrapdoor => "ON_TOP_OF_TRAPDOOR", Self::BigMobsCloseToDanger => "BIG_MOBS_CLOSE_TO_DANGER",
        }
    }
    /// Parse a category name. An unknown category is not silently Blocked.
    pub fn parse(name: &str) -> Option<Self> {
        [Self::Blocked, Self::Open, Self::Walkable, Self::WalkableDoor, Self::Trapdoor,
         Self::PowderSnow, Self::OnTopOfPowderSnow, Self::Fence, Self::Lava, Self::Water,
         Self::WaterBorder, Self::Rail, Self::UnpassableRail, Self::FireInNeighbor, Self::Fire,
         Self::DamagingInNeighbor, Self::Damaging, Self::DoorOpen, Self::DoorWoodClosed,
         Self::DoorIronClosed, Self::Breach, Self::Leaves, Self::StickyHoney, Self::Cocoa,
         Self::DamageCautious, Self::OnTopOfTrapdoor, Self::BigMobsCloseToDanger]
            .into_iter().find(|kind| kind.name() == name)
    }
}

/// The state and borrowed surroundings of a path-node query. Entity identity
/// is optional: a static query has no mob, like Minecraft's nullable-mob form.
pub struct PathNodeTypeContext<'a> {
    pub state: BlockState,
    pub pos: BlockPos,
    pub view: &'a dyn BlockView,
    pub entity: Option<i32>,
    pub kind: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn query_preserves_explicit_blocked_and_fallback_and_allows_registry_reentry() {
        use super::super::{BlockBehavior, BlockBehaviors, MemoryBlockView, Identifier, path_node_type};
        BlockBehaviors::register(BlockBehavior::new(Identifier::new("path_test", "node").unwrap())
            .path_node_type(|context| {
                // Registration here proves consumer code runs outside the
                // behaviour registry's mutex, just like the other callbacks.
                BlockBehaviors::register(BlockBehavior::new(Identifier::new("path_test", "nested").unwrap()));
                assert!(context.view.is_air([3, 64, 5]));
                if context.entity == Some(9) { Some(PathNodeType::Blocked) }
                else if context.pos == [3, 64, 5] { Some(PathNodeType::StickyHoney) }
                else { None }
            }));
        let view = MemoryBlockView::default();
        let mut context = PathNodeTypeContext { state: BlockState::parse("path_test:node").unwrap(),
            pos: [3, 64, 5], view: &view, entity: None, kind: None };
        assert_eq!(path_node_type(&context), Some(PathNodeType::StickyHoney));
        context.entity = Some(9);
        assert_eq!(path_node_type(&context), Some(PathNodeType::Blocked));
        context.entity = None;
        context.pos = [0, 0, 0];
        assert_eq!(path_node_type(&context), None);
        context.state = BlockState::parse("path_test:unregistered").unwrap();
        assert_eq!(path_node_type(&context), None);
    }
}
