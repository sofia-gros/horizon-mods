// One native NBT -> shared semantic component conversion for Pluto consumers.
// This is read/predicate data, never retained-wire identity or a commit token.
use crate::pluto_inventory::advancements::predicates::{
    ComponentText, ComponentValue, ItemSnapshot,
};

pub fn from_nbt(tag: horizon_world::nbt::Tag) -> ComponentValue {
    use horizon_world::nbt::Tag as Native;
    use ComponentValue as Value;
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

/// The exact inverse of `from_nbt`, for writing a provider's component tree
/// back into the game's tag form (component patches, assembled results).
pub fn to_nbt(value: ComponentValue) -> horizon_world::nbt::Tag {
    use horizon_world::nbt::{JavaString, Tag as Native};
    match value {
        ComponentValue::End => Native::End,
        ComponentValue::Byte(v) => Native::Byte(v),
        ComponentValue::Short(v) => Native::Short(v),
        ComponentValue::Int(v) => Native::Int(v),
        ComponentValue::Long(v) => Native::Long(v),
        ComponentValue::Float(v) => Native::Float(v),
        ComponentValue::Double(v) => Native::Double(v),
        ComponentValue::ByteArray(v) => Native::ByteArray(v),
        ComponentValue::String(v) => Native::String(JavaString(v.0)),
        ComponentValue::List(v) => Native::List(v.into_iter().map(to_nbt).collect()),
        ComponentValue::Compound(v) => Native::Compound(
            v.into_iter()
                .map(|(key, v)| (JavaString(key.0), to_nbt(v)))
                .collect(),
        ),
        ComponentValue::IntArray(v) => Native::IntArray(v),
        ComponentValue::LongArray(v) => Native::LongArray(v),
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
            // The host added this field after the legacy source was written.
            // It stays unprojected here, exactly as the host's own general
            // `item_view` constructor leaves it; `ItemSnapshot` does not
            // project it, and component facts flow through `components`.
            custom_data: None,
        };
        // Opaque retained wire may carry facts the saved component codec cannot
        // represent, even after its patch becomes stale. The shared query also
        // checks nested ItemStacks; do not call a partial encoded tree empty.
        let components = if crate::pluto_stack_store::contains_retained_wire(stack) {
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
