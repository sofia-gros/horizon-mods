//! The heads-up display: what a mod draws over the game while no screen is up (Minecraft:
//! `RenderGuiOverlayEvent`, the hotbar's neighbourhood). A mod registers one draw callback;
//! PlutoLib calls it each frame the HUD shows, with the same [`ScreenCanvas`] the menu screens
//! draw on, the window's size, and the local player's own carrying — the inventory the client
//! last saw synced, the equipment it wears, and the player's active effects. The original's modifier HUD (the shield strap's
//! item above the offhand slot, the sleeves' item beside it, the item frame's grid in a corner)
//! is drawn from exactly this: worn tools and what their modifier inventories hold, all of it
//! readable from the stacks themselves on the client.

use std::sync::{Arc, Mutex};

use super::block_entity_api_v1::ItemSlot;
use super::screen_handler_api_v1::ScreenCanvas;

/// What the local player wears and holds, as the client last saw it synced.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HudEquipment {
    /// The helmet slot's stack.
    pub head: Option<ItemSlot>,
    /// The chestplate slot's stack.
    pub chest: Option<ItemSlot>,
    /// The leggings slot's stack.
    pub legs: Option<ItemSlot>,
    /// The boots slot's stack.
    pub feet: Option<ItemSlot>,
    /// The off hand's stack.
    pub offhand: Option<ItemSlot>,
    /// The main hand's stack.
    pub main_hand: Option<ItemSlot>,
    /// Whether the player's main arm is the right one (the hotbar's offhand side flips with it).
    pub right_handed: bool,
}

/// One of the local player's active effects, as the HUD needs it for the original's effect shift:
/// whether it is beneficial and whether it shows (its icon in the GUI).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudEffect {
    /// Whether the effect's category is beneficial (a harmful or neutral one is not).
    pub beneficial: bool,
    /// Whether the effect shows in the GUI: not ambient (a beacon or conduit effect hides its
    /// icon). The host's effect read carries no icon flag, so an effect hidden with showIcon
    /// still counts here; vanilla never hides one that way.
    pub visible: bool,
}

/// What a HUD draw callback receives.
pub struct HudDraw<'a> {
    /// Where to draw, in GUI pixels from the window's top left.
    pub canvas: &'a mut dyn ScreenCanvas,
    /// The window's size in GUI pixels.
    pub size: [f32; 2],
    /// The local player's inventory as the client last saw it synced: the hotbar first, then
    /// the main inventory (36 slots, Minecraft's own order). Empty when the client has not
    /// seen a sync yet.
    pub inventory: &'a [Option<ItemSlot>],
    /// What the local player wears and holds.
    pub equipment: &'a HudEquipment,
    /// Whether the local player is a spectator (the original draws none of its HUD then).
    pub spectator: bool,
    /// The local player's active effects, as the HUD needs them for the original's effect
    /// shift (the minimap and the item frame grid move below the icons at the top right).
    pub effects: &'a [HudEffect],
}

type Draw = Arc<dyn Fn(&mut HudDraw<'_>) + Send + Sync>;

static DRAWS: Mutex<Vec<Draw>> = Mutex::new(Vec::new());

/// Registers HUD draw callbacks (one per mod; registering again adds another).
pub struct HudRegistry;

impl HudRegistry {
    /// Gives the HUD a draw callback, called each frame the HUD shows.
    pub fn register(draw: impl Fn(&mut HudDraw<'_>) + Send + Sync + 'static) {
        if let Ok(mut held) = DRAWS.lock() {
            held.push(Arc::new(draw));
        }
    }
}

/// Whether any HUD draw callback is registered.
pub(crate) fn has_hud() -> bool {
    DRAWS.lock().is_ok_and(|held| !held.is_empty())
}

/// Runs the registered HUD draw callbacks; false when none are registered.
pub(crate) fn draw_hud(canvas: &mut dyn ScreenCanvas, size: [f32; 2], inventory: &[Option<ItemSlot>], equipment: &HudEquipment, spectator: bool, effects: &[HudEffect]) -> bool {
    let draws = DRAWS.lock().map(|held| held.clone()).unwrap_or_default();
    if draws.is_empty() {
        return false;
    }
    let mut draw = HudDraw { canvas, size, inventory, equipment, spectator, effects };
    for each in &draws {
        each(&mut draw);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pure::base::Identifier;

    #[derive(Default)]
    struct Recorder {
        fills: Vec<([f32; 4], u32)>,
    }

    impl ScreenCanvas for Recorder {
        fn fill(&mut self, rect: [f32; 4], argb: u32) {
            self.fills.push((rect, argb));
        }
        fn sprite(&mut self, _: &str, _: [f32; 4], _: [f32; 4]) -> bool {
            true
        }
        fn text(&mut self, _: &str, _: [f32; 2], _: u32, _: bool) {}
        fn text_width(&self, _: &str) -> f32 {
            0.0
        }
    }

    #[test]
    fn a_registered_hud_draws_with_the_window_size_and_the_players_carrying() {
        let mut canvas = Recorder::default();
        assert!(!draw_hud(&mut canvas, [320.0, 240.0], &[], &HudEquipment::default(), false, &[]), "nothing registered yet");
        HudRegistry::register(|draw| {
            assert_eq!(draw.size, [320.0, 240.0]);
            assert_eq!(draw.inventory.len(), 36);
            assert_eq!(draw.equipment.legs.as_ref().map(|stack| stack.count), Some(1));
            assert_eq!(draw.effects, &[HudEffect { beneficial: false, visible: true }]);
            draw.canvas.fill([draw.size[0] / 2.0 - 117.0, draw.size[1] - 41.0, 22.0, 22.0], 0xFF00_00FF);
        });
        assert!(has_hud());
        let mut inventory = vec![None; 36];
        inventory[0] = Some(ItemSlot::new(Identifier::parse("minecraft:stone").unwrap(), 1));
        let equipment = HudEquipment { legs: Some(ItemSlot::new(Identifier::parse("tconstruct:plate_leggings").unwrap(), 1)), right_handed: true, ..HudEquipment::default() };
        let effects = [HudEffect { beneficial: false, visible: true }];
        assert!(draw_hud(&mut canvas, [320.0, 240.0], &inventory, &equipment, false, &effects));
        assert_eq!(canvas.fills, vec![([43.0, 199.0, 22.0, 22.0], 0xFF00_00FF)]);
    }
}
