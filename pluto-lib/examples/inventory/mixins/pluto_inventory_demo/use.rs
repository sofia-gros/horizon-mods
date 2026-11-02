fn inventory_demo_use(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
) -> std::ops::ControlFlow<Result<horizon_client::first_person::UseResult>> {
    use std::ops::ControlFlow::{Break, Continue};
    if game.remote.is_some() || stack.item.as_ref() != "minecraft:stick" {
        return Continue(());
    }
    if horizon_client::pluto_inventory_demo_shared::run(&mut game.inventory) {
        crate::game::pluto_inventory_demo::mark();
        eprintln!("Pluto inventory demo: all planner checks passed");
        Break(Ok(horizon_client::first_person::UseResult::Success {
            swing: true,
        }))
    } else {
        Continue(())
    }
}
