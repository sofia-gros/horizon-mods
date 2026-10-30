use horizon_items::items::ItemStack;

/// Apply a name only to a newly assembled result. An empty field keeps the
/// normal material-derived name.
pub(super) fn name_result(stack: &mut ItemStack, name: Option<&str>) {
    if let Some(name) = name.filter(|name| !name.is_empty()) {
        stack.data_mut().custom_name = Some(name.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_only_nonempty_results() {
        let mut tool = ItemStack::new("tconstruct:pickaxe", 1);
        name_result(&mut tool, None);
        assert_eq!(tool.custom_name(), None);
        name_result(&mut tool, Some(""));
        assert_eq!(tool.custom_name(), None);
        name_result(&mut tool, Some("Mossy pick"));
        assert_eq!(tool.custom_name(), Some("Mossy pick"));
    }
}
