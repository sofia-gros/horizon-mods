// Opt-in demo guard: allow seeding slot 44 with eight, then reject a split to four.
fn demo_creative_slot(slot: i32, count: i32, cancel: &mut bool) {
    if std::env::var_os("PLUTO_DEMO_TEST_CREATIVE_GUARD").is_some() && slot == 44 && count == 4 {
        *cancel = true;
    }
}
