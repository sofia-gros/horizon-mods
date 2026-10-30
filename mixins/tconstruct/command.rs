fn tconstruct_command(game: &mut crate::game::Game<'_>, command: &str) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    const PARTS: [&str; 3] = ["tconstruct:flint_pick_head", "tconstruct:wood_binding", "tconstruct:wood_handle"];
    match command {
        "tconstruct parts" => {
            // Work on a copy so missing ingredients or output capacity cannot consume inputs.
            let mut next = game.inventory.clone();
            let mut available = true;
            for (item, mut needed) in [("minecraft:flint", 1), ("minecraft:stick", 2)] {
                for slot in &mut next {
                    if let Some(stack) = slot.as_mut().filter(|s| &*s.item == item) {
                        let used = stack.count.min(needed);
                        stack.count -= used;
                        needed -= used;
                        if stack.count == 0 { *slot = None; }
                        if needed == 0 { break; }
                    }
                }
                if needed != 0 { available = false; break; }
            }
            if available && next.iter().filter(|slot| slot.is_none()).count() >= PARTS.len() {
                for item in PARTS {
                    let slot = next.iter().position(Option::is_none).unwrap();
                    next[slot] = Some(horizon_client::items::ItemStack::new(item, 1));
                }
                game.inventory = next;
            }
            Break(true)
        }
        "tconstruct assemble" => {
            let slots: Vec<_> = PARTS.iter().filter_map(|id| game.inventory.iter().position(|s| s.as_ref().is_some_and(|s| &*s.item == *id))).collect();
            if slots.len() == 3 && (slots.iter().any(|&i| game.inventory[i].as_ref().unwrap().count == 1) || game.inventory.iter().any(Option::is_none)) {
                for slot in slots.iter().copied() {
                    let stack = game.inventory[slot].as_mut().unwrap();
                    stack.count -= 1;
                    if stack.count == 0 { game.inventory[slot] = None; }
                }
                let freed = slots.iter().copied().find(|&i| game.inventory[i].is_none());
                if let Some(slot) = freed {
                    game.inventory[slot] = Some(horizon_client::items::ItemStack::new("tconstruct:flint_pickaxe", 1));
                } else if let Some(slot) = game.inventory.iter().position(Option::is_none) {
                    game.inventory[slot] = Some(horizon_client::items::ItemStack::new("tconstruct:flint_pickaxe", 1));
                }
            }
            Break(true)
        }
        _ => Continue(()),
    }
}
