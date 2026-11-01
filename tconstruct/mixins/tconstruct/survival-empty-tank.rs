// Source NoContainerIngredient rejects a filled tank, so crafting a Melter
// cannot silently discard the fluid stored on a broken gauge item.
use horizon_world::nbt::Tag;

pub(super) fn empty(tank: Option<&Tag>) -> bool {
    match tank {
        None => true,
        Some(tag @ Tag::Compound(_)) => matches!(tag.get("amount"), Some(Tag::Int(0))) && tag.get("fluid").is_none(),
        Some(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_and_filled_tanks_are_distinguished() {
        assert!(empty(None));
        let empty_tank = Tag::compound([("amount", Tag::Int(0))]);
        assert!(empty(Some(&empty_tank)));
        let filled = Tag::compound([("amount", Tag::Int(1000)), ("fluid", Tag::String("minecraft:lava".into()))]);
        assert!(!empty(Some(&filled)));
        let inconsistent = Tag::compound([("amount", Tag::Int(0)), ("fluid", Tag::String("minecraft:lava".into()))]);
        assert!(!empty(Some(&inconsistent)));
        assert!(!empty(Some(&Tag::Int(0))));
    }
}
