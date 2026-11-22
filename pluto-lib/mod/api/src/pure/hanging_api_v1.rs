//! Fancy hanging entities: item frame and armor stand variants (the shape of
//! the original's `FancyItemFrameEntity` / `FancyArmorStandEntity` classes).
//!
//! The host keeps every item frame in one list and every armor stand in
//! another, both under the vanilla type ids, so a variant cannot be a derived
//! entity type of its own: it is a row in this registry, keyed by the item
//! that places it and by the variant index the entity saves (the original
//! saves the enum ordinal as an int, so the index is part of the format).
//! PlutoLib's game side reads the registry through its transport provider and
//! runs the mechanics below against the host's vanilla frame and stand
//! state; a variant the registry does not know keeps the vanilla behaviour.
//!
//! The mechanics are the original's, as pure functions of the variant row:
//! rotation caps and wraps (`setRotation`), the comparator output
//! (`getAnalogOutput`), the rotation timers (`tick`: gold steps forward every
//! 20 ticks, reversed gold steps back, diamond winds down one step every 300
//! ticks and clicks), `setItem` resetting a rotating frame's rotation, the
//! clear frame using the block behind it and hiding while filled, the
//! netherite frame's fire and explosion immunity, and for stands the
//! placement properties (`StandType.onPlace`), fire immunity at the class
//! level, the clear stand hiding while it wears anything, and the break drop
//! keeping the variant item with its custom name.
use super::base::Identifier;
use std::{collections::BTreeMap, sync::Mutex};

/// One item frame variant: the behaviour of one `FrameType` row, registered
/// by the item that places it. `variant` is the saved ordinal. A variant with
/// a `rotation_step` (gold +1, reversed gold -1) turns on its 20-tick timer;
/// a `winds_down` variant (diamond) instead loses one rotation step every
/// 300 ticks and plays the click sound, and caps its rotation at 16 where
/// the other 16-step variants wrap. `more_rotations` is the original's
/// `hasMoreRotations`: 16 rotation steps and a 0-16 comparator output instead
/// of 8 steps and `rotation % 8 + 1`.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameVariant {
    /// The item that places this variant.
    pub item: Identifier,
    /// The entity type the variant belongs to (the original's entity id).
    pub entity_type: Identifier,
    /// The saved variant index (the original's enum ordinal).
    pub variant: i32,
    /// The rotation change the 20-tick timer applies; 0 for variants whose
    /// timer does not step (diamond winds down instead, the rest never turn).
    pub rotation_step: i32,
    /// Diamond's behaviour: the timer winds the rotation down toward 0.
    pub winds_down: bool,
    /// 16 rotation steps and a 0-16 comparator output instead of 8.
    pub more_rotations: bool,
    /// The frame and the item in it render full bright (manyullyn).
    pub fullbright: bool,
    /// The frame's own model hides while it holds an item (clear).
    pub hidden_when_filled: bool,
    /// Clicking the filled frame uses the block behind it first (clear).
    pub click_through: bool,
    /// Immune to fire and explosions (netherite).
    pub fire_immune: bool,
}

impl FrameVariant {
    /// The original's `doesRotate`: the variant has a rotation timer (it
    /// steps on the 20-tick timer or winds down on the 300-tick one), so
    /// `setItem` resets its rotation and its timer.
    pub fn rotates(&self) -> bool {
        self.rotation_step != 0 || self.winds_down
    }

    /// `FancyItemFrameEntity.setRotation` without the timer reset and the
    /// comparator update (the game side does both): a 16-step variant caps
    /// at 16 when it winds down and wraps modulo 16 otherwise; every other
    /// variant wraps modulo 8. Java's `%` is the remainder, so a negative
    /// input stays negative, exactly as the original leaves it.
    pub fn set_rotation(&self, rotation_in: i32) -> i32 {
        if self.more_rotations {
            if self.winds_down {
                rotation_in.min(16)
            } else {
                rotation_in % 16
            }
        } else {
            rotation_in % 8
        }
    }

    /// `FancyItemFrameEntity.getAnalogOutput`: an empty frame reads 0; a
    /// 16-step variant reads its rotation (0-16, so all sixteen steps are
    /// distinct signals); every other variant reads `rotation % 8 + 1`.
    pub fn analog_output(&self, filled: bool, rotation: i32) -> i32 {
        if !filled {
            return 0;
        }
        if self.more_rotations {
            rotation
        } else {
            rotation % 8 + 1
        }
    }

    /// One tick of the rotation timer (`FancyItemFrameEntity.tick`): returns
    /// the new timer, the rotation to set when the timer fires (the game
    /// side applies it through the same path as a player's rotation, which
    /// also resets the timer and updates the comparators), and whether the
    /// change plays the variant's click sound (diamond's `setRotation`
    /// plays `tconstruct:item_frame_click` whenever the rotation is set
    /// server-side with a comparator update, the wind-down included).
    /// A variant that does not rotate returns its timer untouched.
    pub fn tick_rotation(&self, timer: i32, rotation: i32) -> (i32, Option<i32>, bool) {
        if self.winds_down {
            let timer = timer + 1;
            if timer >= 300 {
                // The timer resets whether or not a step remains; the
                // rotation never winds past 0.
                if rotation > 0 {
                    return (0, Some(self.set_rotation(rotation - 1)), true);
                }
                return (0, None, false);
            }
            return (timer, None, false);
        }
        if self.rotation_step != 0 {
            let timer = timer + 1;
            if timer >= 20 {
                let mut next = rotation + self.rotation_step;
                // The original maps -1 to 7 by hand before setRotation,
                // whose % 8 would keep the negative.
                if next == -1 {
                    next = 7;
                }
                return (0, Some(self.set_rotation(next)), false);
            }
            return (timer, None, false);
        }
        (timer, None, false)
    }
}

/// One armor stand variant: the behaviour of one `StandType` row, registered
/// by the item that places it. `variant` is the saved ordinal. Every variant
/// is fire immune (the class overrides `fireImmune`); `small` and `arms` are
/// the `onPlace` properties; a `hidden_when_equipped` variant (clear) is
/// invisible while any equipment slot is filled.
#[derive(Clone, Debug, PartialEq)]
pub struct StandVariant {
    /// The item that places this variant.
    pub item: Identifier,
    /// The entity type the variant belongs to (the original's entity id).
    pub entity_type: Identifier,
    /// The saved variant index (the original's enum ordinal).
    pub variant: i32,
    /// Placed small (bamboo).
    pub small: bool,
    /// Placed with arms shown (bone, necrotic bone).
    pub arms: bool,
    /// Renders full bright (necrotic bone).
    pub fullbright: bool,
    /// Invisible while any equipment slot is filled (clear).
    pub hidden_when_equipped: bool,
}

impl StandVariant {
    /// `FancyArmorStandEntity.isClearHidden`: a hidden-when-equipped
    /// variant is invisible exactly while it wears anything.
    pub fn hidden(&self, any_equipment: bool) -> bool {
        self.hidden_when_equipped && any_equipment
    }
}

static FRAMES: Mutex<BTreeMap<String, FrameVariant>> = Mutex::new(BTreeMap::new());
static FRAME_VARIANTS: Mutex<BTreeMap<i32, FrameVariant>> = Mutex::new(BTreeMap::new());
static STANDS: Mutex<BTreeMap<String, StandVariant>> = Mutex::new(BTreeMap::new());
static STAND_VARIANTS: Mutex<BTreeMap<i32, StandVariant>> = Mutex::new(BTreeMap::new());

/// The hanging variant registry. One variant per item and one per saved
/// variant index within each kind (frame, stand): the index is written into
/// the entity's saved data, so two registrations may not share one.
pub struct HangingRegistry;

impl HangingRegistry {
    /// Registers one item frame variant. A second registration for the same
    /// item or the same variant index rejects.
    pub fn register_frame(variant: FrameVariant) -> Result<(), String> {
        let mut frames = FRAMES.lock().map_err(|_| "hanging registry poisoned")?;
        let mut variants = FRAME_VARIANTS.lock().map_err(|_| "hanging registry poisoned")?;
        let id = variant.item.to_string();
        if frames.contains_key(&id) {
            return Err(format!("frame variant for {id} already registered"));
        }
        if variants.contains_key(&variant.variant) {
            return Err(format!("frame variant index {} already registered", variant.variant));
        }
        variants.insert(variant.variant, variant.clone());
        frames.insert(id, variant);
        Ok(())
    }

    /// Registers one armor stand variant. A second registration for the same
    /// item or the same variant index rejects.
    pub fn register_stand(variant: StandVariant) -> Result<(), String> {
        let mut stands = STANDS.lock().map_err(|_| "hanging registry poisoned")?;
        let mut variants = STAND_VARIANTS.lock().map_err(|_| "hanging registry poisoned")?;
        let id = variant.item.to_string();
        if stands.contains_key(&id) {
            return Err(format!("stand variant for {id} already registered"));
        }
        if variants.contains_key(&variant.variant) {
            return Err(format!("stand variant index {} already registered", variant.variant));
        }
        variants.insert(variant.variant, variant.clone());
        stands.insert(id, variant);
        Ok(())
    }

    /// The frame variant an item places, if any.
    pub fn frame_for_item(item: &Identifier) -> Option<FrameVariant> {
        FRAMES.lock().ok()?.get(&item.to_string()).cloned()
    }

    /// The frame variant a saved variant index names, if any.
    pub fn frame_for_variant(variant: i32) -> Option<FrameVariant> {
        FRAME_VARIANTS.lock().ok()?.get(&variant).cloned()
    }

    /// The stand variant an item places, if any.
    pub fn stand_for_item(item: &Identifier) -> Option<StandVariant> {
        STANDS.lock().ok()?.get(&item.to_string()).cloned()
    }

    /// The stand variant a saved variant index names, if any.
    pub fn stand_for_variant(variant: i32) -> Option<StandVariant> {
        STAND_VARIANTS.lock().ok()?.get(&variant).cloned()
    }

    pub(crate) fn has_registrations() -> bool {
        FRAMES.lock().is_ok_and(|frames| !frames.is_empty())
            || STANDS.lock().is_ok_and(|stands| !stands.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(item: &str, variant: i32) -> FrameVariant {
        FrameVariant {
            item: Identifier::parse(item).unwrap(),
            entity_type: Identifier::parse("hanging_test:fancy_item_frame").unwrap(),
            variant,
            rotation_step: 0,
            winds_down: false,
            more_rotations: false,
            fullbright: false,
            hidden_when_filled: false,
            click_through: false,
            fire_immune: false,
        }
    }

    fn gold() -> FrameVariant {
        let mut gold = frame("hanging_test:gold_item_frame", 3);
        gold.rotation_step = 1;
        gold
    }

    fn reversed_gold() -> FrameVariant {
        let mut reversed = frame("hanging_test:reversed_gold_item_frame", 0);
        reversed.rotation_step = -1;
        reversed
    }

    fn diamond() -> FrameVariant {
        let mut diamond = frame("hanging_test:diamond_item_frame", 1);
        diamond.winds_down = true;
        diamond.more_rotations = true;
        diamond
    }

    fn manyullyn() -> FrameVariant {
        let mut manyullyn = frame("hanging_test:manyullyn_item_frame", 2);
        manyullyn.more_rotations = true;
        manyullyn.fullbright = true;
        manyullyn
    }

    #[test]
    fn rotation_caps_and_wraps_match_the_original() {
        // Eight-step variants wrap modulo 8, Java remainder included.
        assert_eq!(gold().set_rotation(7), 7);
        assert_eq!(gold().set_rotation(8), 0);
        assert_eq!(gold().set_rotation(9), 1);
        // Sixteen-step variants wrap modulo 16...
        assert_eq!(manyullyn().set_rotation(15), 15);
        assert_eq!(manyullyn().set_rotation(16), 0);
        assert_eq!(manyullyn().set_rotation(17), 1);
        // ...except the winding-down one, which caps at 16 (16 is a real
        // rotation: it renders as a full turn and reads 16 on a comparator).
        assert_eq!(diamond().set_rotation(15), 15);
        assert_eq!(diamond().set_rotation(16), 16);
        assert_eq!(diamond().set_rotation(17), 16);
    }

    #[test]
    fn analog_output_matches_the_original() {
        assert_eq!(gold().analog_output(false, 5), 0, "an empty frame reads 0");
        assert_eq!(gold().analog_output(true, 0), 1);
        assert_eq!(gold().analog_output(true, 7), 8);
        assert_eq!(manyullyn().analog_output(true, 0), 0, "a filled 16-step frame can also read 0");
        assert_eq!(manyullyn().analog_output(true, 9), 9);
        assert_eq!(diamond().analog_output(true, 16), 16);
    }

    #[test]
    fn gold_steps_forward_every_twenty_ticks() {
        let gold = gold();
        assert_eq!(gold.tick_rotation(0, 3), (1, None, false));
        assert_eq!(gold.tick_rotation(19, 3), (0, Some(4), false));
        assert_eq!(gold.tick_rotation(19, 7), (0, Some(0), false), "the step wraps through set_rotation");
        assert!(!gold.winds_down && gold.rotates());
    }

    #[test]
    fn reversed_gold_steps_back_and_maps_minus_one_to_seven() {
        let reversed = reversed_gold();
        assert_eq!(reversed.tick_rotation(19, 3), (0, Some(2), false));
        assert_eq!(reversed.tick_rotation(19, 0), (0, Some(7), false));
        assert_eq!(reversed.tick_rotation(19, 1), (0, Some(0), false));
    }

    #[test]
    fn diamond_winds_down_every_three_hundred_ticks_and_clicks() {
        let diamond = diamond();
        assert_eq!(diamond.tick_rotation(0, 5), (1, None, false));
        assert_eq!(diamond.tick_rotation(299, 5), (0, Some(4), true), "the wind-down step plays the click");
        assert_eq!(diamond.tick_rotation(299, 0), (0, None, false), "the timer still resets at the floor");
        assert_eq!(diamond.tick_rotation(299, 16), (0, Some(15), true));
        assert!(diamond.rotates());
        assert!(!manyullyn().rotates(), "manyullyn has no timer");
    }

    #[test]
    fn the_registry_rejects_duplicate_items_and_indices() {
        HangingRegistry::register_frame(gold()).unwrap();
        assert_eq!(HangingRegistry::frame_for_item(&Identifier::parse("hanging_test:gold_item_frame").unwrap()), Some(gold()));
        assert_eq!(HangingRegistry::frame_for_variant(3), Some(gold()));
        assert!(HangingRegistry::register_frame(gold()).is_err(), "the same item twice rejects");
        let mut clash = frame("hanging_test:other_item_frame", 3);
        clash.rotation_step = 1;
        assert!(HangingRegistry::register_frame(clash).is_err(), "the same saved index twice rejects");
        assert_eq!(HangingRegistry::frame_for_variant(4), None);
        assert_eq!(HangingRegistry::frame_for_item(&Identifier::parse("hanging_test:unknown").unwrap()), None);
    }

    #[test]
    fn stand_variants_hide_only_while_equipped() {
        let clear = StandVariant {
            item: Identifier::parse("hanging_test:clear_armor_stand").unwrap(),
            entity_type: Identifier::parse("hanging_test:armor_stand").unwrap(),
            variant: 3,
            small: false,
            arms: false,
            fullbright: false,
            hidden_when_equipped: true,
        };
        assert!(clear.hidden(true));
        assert!(!clear.hidden(false));
        HangingRegistry::register_stand(clear.clone()).unwrap();
        assert_eq!(HangingRegistry::stand_for_item(&clear.item), Some(clear.clone()));
        assert_eq!(HangingRegistry::stand_for_variant(3), Some(clear));
        assert!(HangingRegistry::register_stand(StandVariant { variant: 4, ..StandVariant {
            item: Identifier::parse("hanging_test:clear_armor_stand").unwrap(),
            entity_type: Identifier::parse("hanging_test:armor_stand").unwrap(),
            variant: 3, small: false, arms: false, fullbright: false, hidden_when_equipped: true,
        }}).is_err(), "the same item twice rejects");
    }
}
