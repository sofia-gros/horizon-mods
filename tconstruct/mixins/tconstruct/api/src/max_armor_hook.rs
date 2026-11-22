//! Connects the max armor attribute and armor level modifiers to PlutoLib's player tick: each tick the worn pieces' claims are read, the
//! largest level of each modifier counts once, and the amount is handed to PlutoLib to add to the wearer's attribute base (the host has no
//! transient attribute modifier); the armor level totals are kept for the modules that read them. SlimeKnights (MIT; see LICENSE).
use crate::{break_speed_hook::with_held_tool, max_armor, tool_stack};
use pluto_lib::{Identifier, ServerLivingEntityEvents};
use std::{collections::BTreeMap, sync::Mutex};

/// The armor level totals of each player (by entity id), as of their last tick.
static LEVELS: Mutex<BTreeMap<i32, BTreeMap<Identifier, i32>>> = Mutex::new(BTreeMap::new());

/// ArmorLevelModule.getLevel: the total of the key over the armor the player wears, 0 when none.
#[allow(dead_code)]
pub(crate) fn armor_level(entity: i32, key: &Identifier) -> i32 {
    LEVELS.lock().ok().and_then(|levels| levels.get(&entity).and_then(|keys| keys.get(key).copied())).unwrap_or(0)
}

/// Registers the callback. Called once from the product's hooks.
pub(crate) fn register() {
    ServerLivingEntityEvents::PLAYER_TICK.register(|tick| {
        let mut claims = Vec::new();
        let mut levels = Vec::new();
        // A broken piece still counts for the modules that allow it; with_held_tool leaves a piece it cannot read alone.
        for (slot, piece) in &tick.armor {
            let Ok((piece_claims, piece_levels)) = with_held_tool(piece, |env, stack| (tool_stack::max_armor_claims(&stack.custom_data, env), tool_stack::armor_levels(&stack.custom_data, env, slot))) else { continue; };
            claims.extend(piece_claims);
            levels.extend(piece_levels);
        }
        tick.attribute_additions.borrow_mut().extend(max_armor::strongest(claims));
        if let Ok(mut all) = LEVELS.lock() {
            let totals = max_armor::totals(levels);
            if totals.is_empty() { all.remove(&tick.entity); } else { all.insert(tick.entity, totals); }
        }
    });
}
