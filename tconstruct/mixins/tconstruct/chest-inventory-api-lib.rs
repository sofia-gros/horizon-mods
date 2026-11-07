// The client library owns the single public re-export.
pub(super) use crate::pluto_inventory::{plan_move, projection::project_slots, StackSnapshot};
pub(super) use crate::pluto_stack_storage::{contains_retained_wire, decode_item_canonical};
