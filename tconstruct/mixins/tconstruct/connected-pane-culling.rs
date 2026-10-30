// The original BetterPaneBlock keeps an exposed vertical arm when the pane
// on the other side does not contain that arm. Keep the IDs per visibility
// instance, including across clones, so vanilla blocks retain their rules.
use std::{cell::RefCell, collections::BTreeSet};
use crate::{collision::CachedCollisionRegistry, registry::BlockStateId};

thread_local! {
    static NEXT_PANES: RefCell<BTreeSet<BlockStateId>> = RefCell::new(BTreeSet::new());
}

fn source_pane(name: &str) -> bool {
    let Some(name) = name.strip_prefix("tconstruct:") else { return false; };
    if matches!(name, "clear_glass_pane" | "soul_glass_pane" | "seared_glass_pane" | "seared_soul_glass_pane" | "scorched_glass_pane" | "scorched_soul_glass_pane") { return true; }
    let Some(color) = name.strip_suffix("_clear_stained_glass_pane") else { return false; };
    matches!(color, "white" | "orange" | "magenta" | "light_blue" | "yellow" | "lime" | "pink" | "gray" | "light_gray" | "cyan" | "purple" | "blue" | "brown" | "green" | "red" | "black")
}

pub(super) fn collect(occlusion: &CachedCollisionRegistry<'_>) {
    let registry = occlusion.registry();
    let panes = (0..registry.len()).filter_map(|id| {
        let state = registry.state(BlockStateId(id as u32))?;
        if source_pane(&state.name) { registry.default_state(&state.name) } else { None }
    }).collect();
    NEXT_PANES.with(|next| *next.borrow_mut() = panes);
}

pub(super) fn initial() -> BTreeSet<BlockStateId> {
    NEXT_PANES.with(|next| std::mem::take(&mut *next.borrow_mut()))
}

pub(super) fn uncovered(current: u8, other: u8) -> bool {
    current & !other != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_source_panes_are_selected() {
        assert!(source_pane("tconstruct:clear_glass_pane"));
        assert!(source_pane("tconstruct:red_clear_stained_glass_pane"));
        assert!(!source_pane("minecraft:glass_pane"));
        assert!(!source_pane("tconstruct:clear_glass"));
        assert!(!source_pane("tconstruct:invented_clear_stained_glass_pane"));
    }
    #[test]
    fn vertical_pane_exposes_arms_missing_from_the_other_pane() {
        assert!(uncovered(0b1100, 0b0100));
        assert!(!uncovered(0b0100, 0b1100));
        assert!(!uncovered(0b1100, 0b1100));
        assert!(!uncovered(0, 0));
    }
}
