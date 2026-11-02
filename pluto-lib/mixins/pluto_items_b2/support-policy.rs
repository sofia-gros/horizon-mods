// Select the first valid candidate in the consumer's order.
pub(crate) fn select<T>(
    candidates: impl IntoIterator<Item = ([i32; 3], T)>,
    mut supported: impl FnMut([i32; 3]) -> bool,
) -> Option<T> {
    candidates
        .into_iter()
        .find_map(|(face, state)| supported(face).then_some(state))
}
#[cfg(test)]
mod tests {
    #[test]
    fn first_supported_candidate_wins() {
        let candidates = [([0, -1, 0], 1), ([0, 1, 0], 2), ([1, 0, 0], 3)];
        let mut checked = Vec::new();
        let result = super::select(candidates, |face| {
            checked.push(face);
            face != [0, -1, 0]
        });
        assert_eq!(result, Some(2));
        assert_eq!(checked, vec![[0, -1, 0], [0, 1, 0]]);
        assert_eq!(super::select(candidates, |_| false), None);
    }
}
