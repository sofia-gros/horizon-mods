fn tconstruct_command(game: &mut crate::game::Game<'_>, command: &str) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    const PARTS: [&str; 3] = ["tconstruct:flint_pick_head", "tconstruct:wood_binding", "tconstruct:wood_handle"];
    match command {
        "tconstruct parts" => {
            let flint = game.inventory.iter().position(|s| s.as_ref().is_some_and(|s| &*s.item == "minecraft:flint"));
            let wood = game.inventory.iter().position(|s| s.as_ref().is_some_and(|s| &*s.item == "minecraft:stick" && s.count >= 2));
            if let (Some(flint), Some(wood)) = (flint, wood) {
                let empty = game.inventory.iter().filter(|s| s.is_none()).count();
                let freed = usize::from(game.inventory[flint].as_ref().unwrap().count == 1)
                    + usize::from(game.inventory[wood].as_ref().unwrap().count == 2);
                if empty + freed >= 3 {
                    for (slot, amount) in [(flint, 1), (wood, 2)] {
                        let stack = game.inventory[slot].as_mut().unwrap();
                        stack.count -= amount;
                        if stack.count == 0 { game.inventory[slot] = None; }
                    }
                    for item in PARTS {
                        let slot = game.inventory.iter().position(Option::is_none).unwrap();
                        game.inventory[slot] = Some(horizon_client::items::ItemStack::new(item, 1));
                    }
                }
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
