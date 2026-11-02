// After a successful raw write, commit the item before fallible events.
pub(crate) fn finish<T>(
    context: &mut T,
    commit_item: impl FnOnce(&mut T) -> bool,
    process_events: impl FnOnce(&mut T) -> bool,
    final_matches: impl FnOnce(&T) -> bool,
) -> (bool, bool, bool) {
    let item_committed = commit_item(context);
    let events_ok = process_events(context);
    let final_state_matches = final_matches(context);
    (item_committed, events_ok, final_state_matches)
}
#[cfg(test)]
mod tests {
    #[derive(Default)]
    struct Fixture {
        item_cost: u8,
        block: &'static str,
    }
    #[test]
    fn accepted_wet_sponge_write_consumes_cost_even_if_host_converts_state() {
        // The raw wet-sponge write was already accepted. The host's Nether
        // on_place changes it into sponge before the final observation.
        let mut fixture = Fixture {
            block: "sponge",
            ..Default::default()
        };
        let result = super::finish(
            &mut fixture,
            |f| {
                f.item_cost += 1;
                true
            },
            |_| true,
            |f| f.block == "wet_sponge",
        );
        assert_eq!(result, (true, true, false));
        assert_eq!(fixture.item_cost, 1);
        assert_eq!(fixture.block, "sponge");
    }
    #[test]
    fn post_write_event_failure_does_not_undo_item_commit() {
        let mut fixture = Fixture::default();
        let result = super::finish(
            &mut fixture,
            |f| {
                f.item_cost += 1;
                true
            },
            |_| false,
            |_| true,
        );
        assert_eq!(result, (true, false, true));
        assert_eq!(fixture.item_cost, 1);
    }
}
