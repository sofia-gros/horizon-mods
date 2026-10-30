// Mantle's cornerless_full names use face-local up, down, left, right order.
// An empty name selects the base sprite rather than a suffix sprite.
pub(crate) fn cornerless_suffix(up: bool, down: bool, left: bool, right: bool) -> String {
    let mut suffix = String::with_capacity(4);
    for (connected, letter) in [(up, 'u'), (down, 'd'), (left, 'l'), (right, 'r')] {
        if connected { suffix.push(letter); }
    }
    suffix
}

#[cfg(test)]
mod tests {
    use super::cornerless_suffix;

    #[test]
    fn names_all_face_local_connection_combinations() {
        let mut names = std::collections::HashSet::new();
        for mask in 0..16 {
            let suffix = cornerless_suffix(mask & 1 != 0, mask & 2 != 0, mask & 4 != 0, mask & 8 != 0);
            assert!(names.insert(suffix));
        }
        assert_eq!(cornerless_suffix(false, false, false, false), "");
        assert_eq!(cornerless_suffix(true, false, true, true), "ulr");
        assert_eq!(cornerless_suffix(true, true, true, true), "udlr");
        assert_eq!(names.len(), 16);
    }
}
