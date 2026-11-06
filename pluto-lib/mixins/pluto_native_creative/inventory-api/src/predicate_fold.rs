//! Lazy three-valued folds. Consumers retain schema and leaf interpretation.
//! None means unknown, not false. Evaluation order and short-circuit are stable.

/// A known true settles the result. Otherwise return unknown if any input was
/// unknown, or false for an exhausted (including empty) iterator.
pub fn any(values: impl IntoIterator<Item = Option<bool>>) -> Option<bool> {
    let mut unknown = false;
    for value in values {
        match value {
            Some(true) => return Some(true),
            Some(false) => {},
            None => unknown = true,
        }
    }
    if unknown { None } else { Some(false) }
}

/// A known false settles the result. Otherwise return unknown if any input was
/// unknown, or true for an exhausted (including empty) iterator.
pub fn all(values: impl IntoIterator<Item = Option<bool>>) -> Option<bool> {
    let mut unknown = false;
    for value in values {
        match value {
            Some(false) => return Some(false),
            Some(true) => {},
            None => unknown = true,
        }
    }
    if unknown { None } else { Some(true) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_identities_are_explicit() {
        assert_eq!(any([]), Some(false));
        assert_eq!(all([]), Some(true));
    }
    #[test]
    fn unknown_is_preserved_unless_a_decisive_value_exists() {
        assert_eq!(any([None, Some(false)]), None);
        assert_eq!(all([None, Some(true)]), None);
        assert_eq!(any([None, Some(true)]), Some(true));
        assert_eq!(all([None, Some(false)]), Some(false));
    }
    #[test]
    fn unneeded_tail_is_never_evaluated() {
        assert_eq!(any(std::iter::once(Some(true)).chain(std::iter::once_with(|| panic!("unvisited")))), Some(true));
        assert_eq!(all(std::iter::once(Some(false)).chain(std::iter::once_with(|| panic!("unvisited")))), Some(false));
    }
}
