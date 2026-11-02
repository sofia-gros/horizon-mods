use horizon_client::items::ItemStack;
use horizon_world::nbt::{JavaString, Tag};
use pluto_item_api_b2::ItemData;

// Store only PlutoLib's own compound, preserving all other custom-data fields.
pub fn read(stack: &ItemStack) -> Result<ItemData, ()> {
    let mut result = ItemData::default();
    let Some(root) = stack.data().and_then(|d| d.other("minecraft:custom_data")) else {
        return Ok(result);
    };
    let Tag::Compound(_) = root else {
        return Err(());
    };
    let Some(owned) = root.get("plutolib:data") else {
        return Ok(result);
    };
    let Tag::Compound(fields) = owned else {
        return Err(());
    };
    for (key, value) in fields {
        let Tag::Long(value) = value else {
            return Err(());
        };
        result.set(key.to_string().map_err(|_| ())?, *value);
    }
    Ok(result)
}
pub fn write(stack: &mut ItemStack, data: &ItemData) -> Result<(), ()> {
    let mut root = match stack.data().and_then(|d| d.other("minecraft:custom_data")) {
        None => Default::default(),
        Some(Tag::Compound(fields)) => fields.clone(),
        _ => return Err(()),
    };
    let owned = Tag::Compound(
        data.iter()
            .map(|(key, value)| (JavaString::from(key), Tag::Long(value)))
            .collect(),
    );
    root.insert(JavaString::from("plutolib:data"), owned);
    stack
        .data_mut()
        .set_other("minecraft:custom_data", Some(Tag::Compound(root)));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_foreign_custom_data_and_rejects_unknown_owned_values() {
        let mut stack = ItemStack::new("example:wand", 1);
        stack.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([("foreign", Tag::Int(7))])),
        );
        let mut data = ItemData::default();
        data.set("example:charge", 3);
        write(&mut stack, &data).unwrap();
        let root = stack
            .data()
            .unwrap()
            .other("minecraft:custom_data")
            .unwrap();
        assert_eq!(root.get("foreign"), Some(&Tag::Int(7)));
        assert_eq!(read(&stack).unwrap().get("example:charge"), Some(3));
        stack.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([(
                "plutolib:data",
                Tag::compound([("unknown", Tag::Int(4))]),
            )])),
        );
        assert!(read(&stack).is_err());
    }
}
