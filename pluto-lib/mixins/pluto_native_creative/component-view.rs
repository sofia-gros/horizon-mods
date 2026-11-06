// One native NBT -> shared semantic component conversion for Pluto consumers.
// This is read/predicate data, never retained-wire identity or a commit token.
use crate::pluto_inventory::advancements::predicates::{
    ComponentText, ComponentValue, ItemSnapshot,
};

pub fn from_nbt(tag: horizon_world::nbt::Tag) -> ComponentValue {
    use ComponentValue as Value;
    use horizon_world::nbt::Tag as Native;
    match tag {
        Native::End => Value::End,
        Native::Byte(v) => Value::Byte(v),
        Native::Short(v) => Value::Short(v),
        Native::Int(v) => Value::Int(v),
        Native::Long(v) => Value::Long(v),
        Native::Float(v) => Value::Float(v),
        Native::Double(v) => Value::Double(v),
        Native::ByteArray(v) => Value::ByteArray(v),
        Native::String(v) => Value::String(ComponentText(v.0)),
        Native::List(v) => Value::List(v.into_iter().map(from_nbt).collect()),
        Native::Compound(v) => Value::Compound(
            v.into_iter()
                .map(|(key, v)| (ComponentText(key.0), from_nbt(v)))
                .collect(),
        ),
        Native::IntArray(v) => Value::IntArray(v),
        Native::LongArray(v) => Value::LongArray(v),
    }
}

/// Structural inverse only. This does not validate a component schema or grant
/// permission to replace a native stack. Callers must bound supplied trees.
pub fn to_nbt(value: &ComponentValue) -> horizon_world::nbt::Tag {
    use ComponentValue as Value;
    use horizon_world::nbt::{JavaString, Tag as Native};
    match value {
        Value::End => Native::End,
        Value::Byte(v) => Native::Byte(*v),
        Value::Short(v) => Native::Short(*v),
        Value::Int(v) => Native::Int(*v),
        Value::Long(v) => Native::Long(*v),
        Value::Float(v) => Native::Float(*v),
        Value::Double(v) => Native::Double(*v),
        Value::ByteArray(v) => Native::ByteArray(v.clone()),
        Value::String(v) => Native::String(JavaString(v.0.clone())),
        Value::List(v) => Native::List(v.iter().map(to_nbt).collect()),
        Value::Compound(v) => Native::Compound(
            v.iter()
                .map(|(key, v)| (JavaString(key.0.clone()), to_nbt(v)))
                .collect(),
        ),
        Value::IntArray(v) => Native::IntArray(v.clone()),
        Value::LongArray(v) => Native::LongArray(v.clone()),
    }
}

pub struct ItemFacts {
    view: crate::advancements::ItemView,
    components: Option<ComponentValue>,
}
impl ItemFacts {
    pub fn capture(stack: &crate::items::ItemStack) -> Self {
        let view = crate::advancements::ItemView {
            id: stack.item.to_string(),
            count: stack.count,
            damage: stack.damage,
            enchantments: stack
                .enchantments()
                .iter()
                .map(|(id, level)| (id.to_string(), *level))
                .collect(),
            potion: stack.potion().map(str::to_owned),
            custom_name: stack.custom_name().is_some(),
        };
        // Opaque retained wire may carry facts the saved component codec cannot
        // represent, even after its patch becomes stale. The shared query also
        // checks nested ItemStacks; do not call a partial encoded tree empty.
        let components = if crate::pluto_stack_storage::contains_retained_wire(stack) {
            None
        } else {
            // Native None means default/empty saved component patch, not an
            // unavailable capture. It does not expand prototype defaults.
            Some(
                crate::item_data::encode_components(stack)
                    .map(from_nbt)
                    .unwrap_or_else(|| ComponentValue::Compound(Default::default())),
            )
        };
        Self { view, components }
    }
    pub fn view(&self) -> ItemSnapshot<'_> {
        ItemSnapshot {
            id: &self.view.id,
            count: self.view.count,
            damage: self.view.damage,
            enchantments: &self.view.enchantments,
            potion: self.view.potion.as_deref(),
            custom_name: self.view.custom_name,
            components: self.components.as_ref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pluto_component_view_preserves_float_bits_and_utf16() {
        use horizon_world::nbt::{JavaString, Tag};
        let value = Tag::List(vec![
            Tag::Float(0x7fc0_0042),
            Tag::Double(0xfff0_0000_0000_0001),
            Tag::String(JavaString(vec![0xd800])),
            Tag::Short(-9),
            Tag::Long(i64::MIN),
        ]);
        assert_eq!(
            from_nbt(value),
            ComponentValue::List(vec![
                ComponentValue::Float(0x7fc0_0042),
                ComponentValue::Double(0xfff0_0000_0000_0001),
                ComponentValue::String(ComponentText(vec![0xd800])),
                ComponentValue::Short(-9),
                ComponentValue::Long(i64::MIN)
            ])
        );
    }
    #[test]
    fn pluto_component_conversion_is_structurally_reversible() {
        use horizon_world::nbt::{JavaString, Tag};
        let values = vec![
            Tag::End,
            Tag::Byte(-8),
            Tag::Short(-9),
            Tag::Int(-10),
            Tag::Long(i64::MIN),
            Tag::Float(0x7fc00001),
            Tag::Double(0xfff0000000000001),
            Tag::ByteArray(vec![0, 255]),
            Tag::String(JavaString(vec![0xd800])),
            Tag::IntArray(vec![i32::MIN]),
            Tag::LongArray(vec![i64::MAX]),
            Tag::List(vec![Tag::Int(7)]),
            Tag::Compound(std::collections::BTreeMap::from([(
                JavaString(vec![0xdc00]),
                Tag::Byte(1),
            )])),
        ];
        for value in values {
            assert_eq!(to_nbt(&from_nbt(value.clone())), value);
        }
    }
    #[test]
    fn pluto_component_view_keeps_known_default_patch_distinct_from_unavailable() {
        let stack = crate::items::ItemStack {
            item: "test:item".into(),
            count: 0,
            damage: 0,
            extra: None,
        };
        let facts = ItemFacts::capture(&stack);
        let view = facts.view();
        assert_eq!((view.id, view.count, view.damage), ("test:item", 0, 0));
        assert_eq!(
            view.components,
            Some(&ComponentValue::Compound(Default::default()))
        );
    }
}
