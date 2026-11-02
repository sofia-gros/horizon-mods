// None without an entry is unknown; an entry with no restriction is authorized.
pub(crate) fn permits(decision: Option<Option<([i32; 3], u32)>>, at: [i32; 3]) -> bool {
    match decision {
        None => false,
        Some(None) => true,
        Some(Some((spawn, radius))) => {
            at[0].abs_diff(spawn[0]).max(at[2].abs_diff(spawn[2])) > radius
        }
    }
}
#[cfg(test)]
mod tests {
    use super::permits;
    #[test]
    fn missing_and_protected_are_rejected() {
        assert!(!permits(None, [0, 64, 0]));
        assert!(permits(Some(None), [0, 64, 0]));
        let restriction = Some(Some(([100, 64, 200], 16)));
        assert!(!permits(restriction, [116, 300, 200]));
        assert!(!permits(restriction, [84, 64, 184]));
        assert!(permits(restriction, [117, -50, 200]));
    }
}
