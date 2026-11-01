// Source TinkerStationBlock slot counts include the tool slot.
pub(super) fn input_capacity(block: &str) -> Option<usize> {
    match block {
        "tconstruct:tinker_station" => Some(3),
        "tconstruct:tinkers_anvil" | "tconstruct:scorched_anvil" => Some(5),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::input_capacity;

    #[test]
    fn source_tables_have_distinct_input_capacities() {
        assert_eq!(input_capacity("tconstruct:tinker_station"), Some(3));
        assert_eq!(input_capacity("tconstruct:tinkers_anvil"), Some(5));
        assert_eq!(input_capacity("tconstruct:scorched_anvil"), Some(5));
        assert_eq!(input_capacity("tconstruct:part_builder"), None);
    }
}
