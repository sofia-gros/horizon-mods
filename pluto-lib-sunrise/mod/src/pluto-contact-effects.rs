// Transient, world-owned contact effects. No globals, saved data or queue
// synchronization. The native sound path remains responsible for delivery.
use crate::attachments::{AttachmentType, Attachments, pluto_attachment_transient};
use crate::registry::BlockStateId;
use super::pluto_contact_pending::{Clock, Pending};

#[derive(Clone)]
pub struct ContactSound {
    pub state: BlockStateId,
    pub pos: [i32; 3],
    pub volume: f32,
    pub pitch: f32,
}
static SOUNDS: AttachmentType<Pending<ContactSound>> = AttachmentType {
    namespace: "pluto_lib", name: "transient_contact_sounds",
    default: Pending::default, codec: None, copy_on_death: false, sync: false,
};
fn initialize(attachments: &mut Attachments) -> bool {
    if attachments.has(&SOUNDS) {
        return pluto_attachment_transient::peek(attachments, &SOUNDS).ok().flatten().is_some();
    }
    attachments.set(&SOUNDS, Pending::default()).is_ok()
}
pub fn queue_player(attachments: &mut Attachments, entity: i32, tick: i32, sound: ContactSound) -> bool {
    if !initialize(attachments) { return false; }
    pluto_attachment_transient::peek(attachments, &SOUNDS).ok().flatten()
        .is_some_and(|pending| pending.push(entity, Clock::Player(tick), sound))
}
pub fn drain_player(attachments: &Attachments, entity: i32, tick: i32) -> Vec<ContactSound> {
    pluto_attachment_transient::peek(attachments, &SOUNDS).ok().flatten()
        .map_or_else(Vec::new, |pending| pending.drain_player(entity, tick))
}
/// True owns the matching flush; false may mean an already-active nested phase.
pub fn begin_world(attachments: &mut Attachments, tick: i64) -> bool {
    if !initialize(attachments) { return false; }
    pluto_attachment_transient::peek(attachments, &SOUNDS).ok().flatten()
        .is_some_and(|pending| pending.begin_world(tick))
}
pub fn world_active(attachments: &Attachments, tick: i64) -> bool {
    pluto_attachment_transient::peek(attachments, &SOUNDS).ok().flatten()
        .is_some_and(|pending| pending.world_active(tick))
}
pub fn queue_world(attachments: &Attachments, entity: i32, tick: i64, sound: ContactSound) -> bool {
    pluto_attachment_transient::peek(attachments, &SOUNDS).ok().flatten()
        .is_some_and(|pending| pending.push(entity, Clock::World(tick), sound))
}
pub fn drain_world(attachments: &Attachments, tick: i64) -> Vec<(i32, ContactSound)> {
    pluto_attachment_transient::peek(attachments, &SOUNDS).ok().flatten()
        .map_or_else(Vec::new, |pending| pending.drain_world(tick))
}
