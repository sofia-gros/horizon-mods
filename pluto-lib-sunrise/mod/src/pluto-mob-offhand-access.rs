// Mob off-hand stack edit transport for the entity reach: an edit of the
// stack in a mob's off hand cannot run through the living handle's
// equipment read (API 382), because that read builds the slot from the
// item id alone — `Mob::equipment_stacks` keeps no stack for the off
// hand, so it answers one item with no damage and no components — while
// the host itself keeps the whole stack a mob holds there in its gear
// store: `mobs::finalize::item_stack` (the host's own
// `LivingEntity.getItemBySlot`) reads it back, and
// `finalize::set_item_stack` (its `setItemSlot`) puts a whole stack
// there, as the `/item` command's mob slot write also does. Reading
// through the handle would hand the edit a stack that is not the mob's,
// and putting the edited copy back would cut the count to one and drop
// every component the edit never named. So an off-hand edit queues here
// instead, and `drain` — woven at the head of the game's own rider
// synchronisation with the entity reach's other transports — applies it
// against the mob itself through that pair: the whole stack as it is
// when the edit lands is read, checked against the item the edit names,
// edited, and put back whole, so the count and the wear that changed in
// between stay.
//
// Acceptance is a request taken, not an edit guaranteed: a mob the game
// no longer holds, a slot since emptied or filled with another item, and
// edits that change nothing are dropped at application. Requests for
// another dimension wait for that dimension's synchronisation.

use std::sync::Mutex;

use super::Game;
use sunrise_world::nbt::{Compound, JavaString, Tag};

/// One queued off-hand edit: the item the edit names and the custom-data
/// edits to apply, in order, to the stack in the mob's off hand. An
/// edit's value is the tag to put at its path, or None to remove the
/// entry there.
pub struct OffhandEdit {
    pub item: String,
    pub edits: Vec<(Vec<String>, Option<Tag>)>,
}

static QUEUE: Mutex<Vec<(String, u32, OffhandEdit)>> = Mutex::new(Vec::new());

/// The most edits held for later application; past it, new edits are refused.
const QUEUE_CAP: usize = 1024;

/// Queues an edit of the off-hand stack of the mob `mob` in `dimension`
/// (see the module header); whether the request was taken. The game
/// re-checks at application time, so acceptance is a request taken, not
/// an edit guaranteed.
pub fn push_edit(dimension: &str, mob: u32, item: String, edits: Vec<(Vec<String>, Option<Tag>)>) -> bool {
    let Ok(mut queue) = QUEUE.lock() else { return false };
    if queue.len() >= QUEUE_CAP {
        return false;
    }
    queue.push((dimension.to_string(), mob, OffhandEdit { item, edits }));
    true
}

/// Applies this game's queued off-hand edits, in request order. Requests
/// for other dimensions stay queued for their own synchronisation.
pub fn drain(game: &mut Game<'_>) {
    let dimension = game.dim.id();
    let mut pending: Vec<(u32, OffhandEdit)> = Vec::new();
    {
        let Ok(mut queue) = QUEUE.lock() else { return };
        let mut index = 0;
        while index < queue.len() {
            if queue[index].0 == dimension {
                let (_, mob, edit) = queue.remove(index);
                pending.push((mob, edit));
            } else {
                index += 1;
            }
        }
    }
    for (mob, edit) in pending {
        apply(game, mob, &edit);
    }
}

fn apply(game: &mut Game<'_>, id: u32, edit: &OffhandEdit) {
    use crate::mobs::finalize;
    let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == id) else { return };
    let Some(mut stack) = finalize::item_stack(mob, finalize::OFFHAND) else { return };
    if &*stack.item != edit.item || stack.count == 0 {
        return;
    }
    let mut data = match stack.data().and_then(|data| data.other("minecraft:custom_data")) {
        Some(Tag::Compound(compound)) => compound.clone(),
        _ => Compound::new(),
    };
    if !edit_custom_data(&mut data, &edit.edits) {
        return;
    }
    stack.data_mut().set_other("minecraft:custom_data", (!data.is_empty()).then(|| Tag::Compound(data)));
    stack.tidy();
    finalize::set_item_stack(mob, finalize::OFFHAND, stack);
}

// The custom-data edit application of the pure entity events
// (`entity_events_v1::edit_custom_data`), written out against the game's
// own NBT: the edits apply in order; an edit with an empty path is
// skipped; one whose path runs through a value that is not a compound
// replaces that value with a compound; a removal whose path does not run
// through compounds already there changes nothing. True when any of the
// edits changed the data.
fn edit_custom_data(data: &mut Compound, edits: &[(Vec<String>, Option<Tag>)]) -> bool {
    let before = data.clone();
    for (path, value) in edits {
        let Some((leaf, parents)) = path.split_last() else { continue };
        let mut at = &mut *data;
        let mut present = true;
        for key in parents {
            let key = JavaString::from(key.as_str());
            if value.is_none() && !matches!(at.get(&key), Some(Tag::Compound(_))) {
                present = false;
                break;
            }
            let entry = at.entry(key).or_insert_with(|| Tag::Compound(Compound::new()));
            if !matches!(entry, Tag::Compound(_)) {
                *entry = Tag::Compound(Compound::new());
            }
            let Tag::Compound(inner) = entry else { unreachable!("made a compound just above") };
            at = inner;
        }
        if !present {
            continue;
        }
        match value {
            Some(value) => {
                at.insert(JavaString::from(leaf.as_str()), value.clone());
            }
            None => {
                at.remove(&JavaString::from(leaf.as_str()));
            }
        }
    }
    *data != before
}
