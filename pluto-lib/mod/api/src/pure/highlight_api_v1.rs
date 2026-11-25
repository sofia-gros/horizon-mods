//! The block outlines a mod draws around the blocks its held tool is
//! about to affect (the original's `ToolRenderEvents.renderBlockHighlights`,
//! over the area-of-effect iterator's DISPLAY predicate): while the local
//! player holds a tool and aims at a block, each provider answers the
//! extra positions its own hooks would highlight — for the brush, the
//! brushable neighbours of the aimed block, for a mining tool the
//! neighbours the tool is effective on — and the game draws the
//! block outline around each of them. The aimed block itself is not a
//! provider's business: the game draws its own outline around the
//! target, as vanilla draws it, and the original's event adds only the
//! extra blocks.
//!
//! In the original the positions are recomputed on every render event
//! from the player's current aim, so a highlight lives exactly as long
//! as the aim that causes it. Here the transport asks every registered
//! provider once a frame while the world is drawn, with the facts of
//! the local player's view at that moment — its block reach for the
//! player's game mode, whether a position sits inside the world
//! border, and the published hardness of the blocks around the
//! target — and draws the merged set:
//! a provider's answer is one frame's outline set, refreshed by being
//! asked again and gone the first frame it stops answering. A provider
//! that owns nothing in the facts answers an empty set.

use std::sync::RwLock;

use super::base::Identifier;
use super::block_entity_api_v1::ItemSlot;

/// The most extra blocks outlined at once, across every provider (the
/// original's `MAX_BLOCKS` in `ToolRenderEvents`): the merged set is
/// cut to this, in the order providers answered.
pub const MAX_HIGHLIGHTS: usize = 60;

/// The outline's colour, as the game draws a targeted block's own:
/// black at 40% opacity, ARGB (vanilla's `renderHitOutline` colour).
pub const OUTLINE_ARGB: u32 = 0x6600_0000;

/// The outline's line width in pixels, as the game draws a targeted
/// block's own.
pub const OUTLINE_WIDTH: f32 = 2.0;

/// The block reach in survival (and adventure and spectator): the
/// `minecraft:block_interaction_range` attribute's base value, as the
/// game recomputes it every tick (`Game::tick`).
pub const SURVIVAL_BLOCK_REACH: f32 = 4.5;

/// The block reach in creative: the same attribute's base value there.
pub const CREATIVE_BLOCK_REACH: f32 = 5.0;

/// The block reach for game mode `mode` (0 survival, 1 creative,
/// 2 adventure, 3 spectator, as the game numbers its modes): creative
/// reaches 5.0, every other mode 4.5.
pub fn block_reach_for_mode(mode: i32) -> f32 {
    if mode == 1 { CREATIVE_BLOCK_REACH } else { SURVIVAL_BLOCK_REACH }
}

/// The block the local player's view rests on, as the game states it
/// when it asks: where it is, the face the view enters it through (a
/// unit offset), what it is, and how far along the view it sits.
#[derive(Clone, Debug, PartialEq)]
pub struct HighlightTarget {
    /// The block's position.
    pub pos: [i32; 3],
    /// The face the view enters the block through, as a unit offset
    /// (the face the area-of-effect iterator orients itself by).
    pub face: [i32; 3],
    /// The block at `pos`, as the game names it.
    pub block: Identifier,
    /// How far along the view the block sits, in blocks.
    pub distance: f32,
}

/// The facts of one frame's outline ask, as the game states them when
/// it asks: the local player's main-hand stack and view, its block
/// reach for the player's game mode, and reads of the blocks around
/// the view's target. A frame on the title screen is never asked; a
/// frame with no block in the view has no target.
pub struct HighlightFacts<'a> {
    /// The stack in the player's main hand (a tool's modifiers and
    /// persistent data ride in its custom data), or None when the
    /// hand is empty or its stack cannot be named.
    pub main_hand: Option<&'a ItemSlot>,
    /// The block the view rests on, or None when it rests on nothing
    /// (the sky, an entity, a block the game cannot name).
    pub target: Option<HighlightTarget>,
    /// The player's yaw, in degrees (0 faces south, 90 west).
    pub yaw: f32,
    /// The player's pitch, in degrees.
    pub pitch: f32,
    /// The block reach the distance gate uses: the
    /// `minecraft:block_interaction_range` attribute's base value for
    /// the player's game mode (4.5 in survival, 5.0 in creative), as
    /// the game recomputes it every tick.
    pub block_reach: f32,
    /// Whether a position sits inside the world border (the original's
    /// `WorldBorder.isWithinBounds`): positions outside it are never
    /// outlined, and do not spend one of the sixty slots.
    pub in_world_border: &'a dyn Fn([i32; 3]) -> bool,
    /// The block at a position near the view, or None when nothing is
    /// loaded there: the read an area-of-effect iterator walks by.
    pub block_at: &'a dyn Fn([i32; 3]) -> Option<Identifier>,
    /// Whether the block at a position is in a block tag.
    pub in_tag_at: &'a dyn Fn([i32; 3], &Identifier) -> bool,
    /// The destroy speed (hardness) of the block at a position, as the
    /// game's own block table publishes it (-1 for a block that cannot
    /// be broken), or None when the table has no answer for the block:
    /// the read the default block predicate's hardness clause compares
    /// a neighbour's hardness against the target's by.
    pub hardness_at: &'a dyn Fn([i32; 3]) -> Option<f32>,
}

/// Answers the extra block positions one mod's hooks highlight under
/// `facts`, or an empty set when they highlight nothing this frame.
pub type HighlightProvider = fn(&HighlightFacts<'_>) -> Vec<[i32; 3]>;

static PROVIDERS: RwLock<Vec<HighlightProvider>> = RwLock::new(Vec::new());

/// The registry of highlight providers. Register once during startup;
/// the transport asks every registered provider on each frame the
/// world is drawn and draws the merged set as block outlines.
pub struct HighlightRegistry;

impl HighlightRegistry {
    /// Registering the same provider again does not register it twice.
    pub fn register(provider: HighlightProvider) {
        if let Ok(mut providers) = PROVIDERS.write() {
            if !providers.contains(&provider) {
                providers.push(provider);
            }
        }
    }
}

/// The merged outline set over `facts`: every registered provider's
/// positions, in the order they answered, each position once, cut to
/// [`MAX_HIGHLIGHTS`]. A position outside the world border never
/// enters the set, and does not spend one of the sixty slots (the
/// original counts only the blocks it draws). Empty when no provider
/// highlights anything.
pub fn highlight_positions(facts: &HighlightFacts<'_>) -> Vec<[i32; 3]> {
    let Ok(providers) = PROVIDERS.read() else { return Vec::new() };
    let mut positions: Vec<[i32; 3]> = Vec::new();
    for provider in providers.iter() {
        for pos in provider(facts) {
            if !positions.contains(&pos) && (facts.in_world_border)(pos) {
                positions.push(pos);
                if positions.len() >= MAX_HIGHLIGHTS {
                    return positions;
                }
            }
        }
    }
    positions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_block(_: [i32; 3]) -> Option<Identifier> {
        None
    }

    fn no_tag(_: [i32; 3], _: &Identifier) -> bool {
        false
    }

    fn no_hardness(_: [i32; 3]) -> Option<f32> {
        None
    }

    fn facts<'a>(main_hand: Option<&'a ItemSlot>, target: Option<HighlightTarget>) -> HighlightFacts<'a> {
        HighlightFacts { main_hand, target, yaw: 0.0, pitch: 0.0, block_reach: CREATIVE_BLOCK_REACH, in_world_border: &|_| true, block_at: &no_block, in_tag_at: &no_tag, hardness_at: &no_hardness }
    }

    fn two(_: &HighlightFacts<'_>) -> Vec<[i32; 3]> {
        vec![[1, 64, -3], [1, 64, -3], [2, 64, -3]]
    }

    fn flood(_: &HighlightFacts<'_>) -> Vec<[i32; 3]> {
        (0..100).map(|x| [x, 64, 0]).collect()
    }

    #[test]
    fn answers_merge_dedupe_and_cap_in_provider_order() {
        let target = HighlightTarget {
            pos: [0, 64, -3],
            face: [0, 0, 1],
            block: Identifier::new("minecraft", "suspicious_sand").unwrap(),
            distance: 2.5,
        };
        assert_eq!(target.face, [0, 0, 1], "the target keeps the face the view entered through");
        assert!(highlight_positions(&facts(None, Some(target.clone()))).is_empty(), "no providers registered yet");
        HighlightRegistry::register(|_| Vec::new());
        assert!(highlight_positions(&facts(None, None)).is_empty(), "a provider that never speaks outlines nothing");
        HighlightRegistry::register(two);
        HighlightRegistry::register(two);
        assert_eq!(highlight_positions(&facts(None, None)), vec![[1, 64, -3], [2, 64, -3]], "registered once, duplicates drawn once");
        HighlightRegistry::register(flood);
        let merged = highlight_positions(&facts(None, None));
        assert_eq!(merged.len(), MAX_HIGHLIGHTS, "the merged set is cut to the original's maximum");
        assert_eq!(merged[..2], [[1, 64, -3], [2, 64, -3]], "earlier providers keep their places at the head");
        assert_eq!(merged[2], [0, 64, 0], "the flood fills the rest in its own order");
        // A position outside the world border is never outlined, and does not
        // spend one of the sixty slots (the original counts only drawn blocks).
        let border = |pos: [i32; 3]| pos[0] <= 1;
        let bordered = HighlightFacts { in_world_border: &border, ..facts(None, None) };
        assert_eq!(highlight_positions(&bordered), vec![[1, 64, -3], [0, 64, 0], [1, 64, 0]],
            "the border lets only the flood's first two blocks through");
        let border = |pos: [i32; 3]| pos[0] >= 2;
        let facts2 = HighlightFacts { in_world_border: &border, ..facts(None, None) };
        let merged = highlight_positions(&facts2);
        assert_eq!(merged.len(), MAX_HIGHLIGHTS, "excluded positions do not spend slots");
        assert_eq!(merged[59], [60, 64, 0], "the sixtieth drawn block is the flood's fifty-ninth");
    }

    #[test]
    fn the_reach_is_the_game_modes_own() {
        assert_eq!(SURVIVAL_BLOCK_REACH, 4.5);
        assert_eq!(CREATIVE_BLOCK_REACH, 5.0);
        assert_eq!(block_reach_for_mode(0), 4.5, "survival");
        assert_eq!(block_reach_for_mode(1), 5.0, "creative");
        assert_eq!(block_reach_for_mode(2), 4.5, "adventure reaches as survival does");
        assert_eq!(block_reach_for_mode(3), 4.5, "spectator reaches as survival does");
    }

    #[test]
    fn the_outline_style_is_the_targeted_blocks_own() {
        assert_eq!(MAX_HIGHLIGHTS, 60);
        assert_eq!(OUTLINE_ARGB >> 24, 0x66, "black at 40% opacity, as vanilla draws the target's outline");
        assert_eq!(OUTLINE_ARGB & 0xFF_FFFF, 0);
        assert_eq!(OUTLINE_WIDTH, 2.0);
    }
}
