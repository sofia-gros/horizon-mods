// Living-entity screen previews a mod's screen asked for during its draw, waiting for the
// flush at the end of that draw (pluto-living-preview): the draw itself holds only the
// frame's batch, while rendering a living entity needs the game.
use std::sync::Mutex;

use sunrise_items::items::ItemStack;

/// One queued preview: the rect (corner coordinates, GUI pixels) to draw into, the entity
/// type's id, the item the figure wears or holds, and how the figure stands.
pub struct QueuedPreview {
    pub rect: [f32; 4],
    pub kind: String,
    pub item: Option<ItemStack>,
    pub yaw: f32,
    pub arms: bool,
    pub base_plate: bool,
}

static QUEUE: Mutex<Vec<QueuedPreview>> = Mutex::new(Vec::new());

/// Queues a preview for the flush at the end of the current screen draw.
pub fn queue(preview: QueuedPreview) {
    if let Ok(mut queue) = QUEUE.lock() {
        queue.push(preview);
    }
}

/// Takes every queued preview, leaving the queue empty.
pub fn take_queued() -> Vec<QueuedPreview> {
    QUEUE.lock().map(|mut queue| std::mem::take(&mut *queue)).unwrap_or_default()
}
