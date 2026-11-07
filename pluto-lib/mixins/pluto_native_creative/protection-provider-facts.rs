// The public provider reads the original borrowed catalog and original captured
// stacks. This adapter never rereads a live Game or manufactures active-use data.
use crate::{
    pluto_component_view::ItemFacts,
    pluto_inventory::{equipment as equipment_api, protection as api},
};
use horizon_world::{
    damage_source::{Catalog, DamageTypeId, pluto_context},
    player_inventory::EquipmentSlot,
};

pub(super) struct Source<'a>(pluto_context::SourceView<'a>);
impl<'a> Source<'a> {
    pub(super) fn new(catalog: &'a Catalog, source: DamageTypeId) -> Self {
        Self(pluto_context::SourceView::new(catalog, source))
    }
}
impl api::Source for Source<'_> {
    fn name(&self) -> Option<&str> {
        self.0.name()
    }
    fn is_type(&self, name: &str) -> Option<bool> {
        self.0.is_type(name)
    }
    fn in_tag(&self, tag: &str) -> Option<bool> {
        self.0.in_tag(tag)
    }
    fn bypasses(&self, kind: api::Bypass) -> Option<bool> {
        use horizon_world::damage_reduction::Bypass;
        self.0.bypasses(match kind {
            api::Bypass::Armor => Bypass::Armor,
            api::Bypass::Effects => Bypass::Effects,
            api::Bypass::Resistance => Bypass::Resistance,
            api::Bypass::Enchantments => Bypass::Enchantments,
        })
    }
}

pub(super) struct Equipment {
    // Outer None is uncaptured, inner None is known empty. ItemFacts is the one
    // shared component converter, not retained stack/commit identity.
    slots: [Option<Option<ItemFacts>>; 6],
}
impl Equipment {
    pub(super) fn capture(snapshot: &crate::pluto_equipment::Snapshot) -> Self {
        use EquipmentSlot::*;
        Self {
            slots: [Head, Chest, Legs, Feet, MainHand, OffHand].map(|slot| {
                snapshot
                    .get(slot)
                    .map(|stack| stack.map(ItemFacts::capture))
            }),
        }
    }
}
impl equipment_api::EquipmentSlots for Equipment {
    fn slot(&self, slot: equipment_api::EquipmentSlot) -> equipment_api::Slot<'_> {
        use equipment_api::{EquipmentSlot::*, Slot};
        match &self.slots[match slot {
            Head => 0,
            Chest => 1,
            Legs => 2,
            Feet => 3,
            MainHand => 4,
            OffHand => 5,
        }] {
            None => Slot::Unavailable,
            Some(None) => Slot::Empty,
            Some(Some(item)) => Slot::Item(item.view()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use api::Source as _;
    use equipment_api::EquipmentSlots as _;
    use serde_json::json;
    fn catalog(names: &[&str]) -> Catalog {
        let types: Vec<_> = names
            .iter()
            .map(|name| {
                json!({
                    "id": name, "message_id":"test", "exhaustion":"3dcccccd",
                    "scaling":"never", "effects":"hurt", "death_message_type":"default"
                })
            })
            .collect();
        Catalog::from_json(
            &json!({"data_version":horizon_world::DATA_VERSION,"types":types}),
            &json!({}),
        )
        .unwrap()
    }
    #[test]
    fn pluto_provider_source_preserves_known_missing_and_unknown() {
        let small = catalog(&["test:a"]);
        let large = catalog(&["test:a", "test:b"]);
        let known = Source::new(&small, small.id("test:a").unwrap());
        assert_eq!(known.name(), Some("test:a"));
        assert_eq!(known.in_tag("test:missing"), Some(false));
        assert_eq!(known.bypasses(api::Bypass::Armor), Some(false));
        let unknown = Source::new(&small, large.id("test:b").unwrap());
        assert_eq!(unknown.name(), None);
        assert_eq!(unknown.in_tag("test:missing"), None);
        assert_eq!(unknown.bypasses(api::Bypass::Armor), None);
    }
    #[test]
    fn pluto_provider_equipment_reads_captured_slots_without_active_use() {
        let item = |id: &str, damage| crate::items::ItemStack {
            item: id.into(),
            count: 1,
            damage,
            extra: None,
        };
        let mut main = item("test:main", 7);
        let off = item("test:off", 3);
        let native = crate::pluto_equipment::Snapshot::capture(
            3,
            "test:world",
            &[None, None, None, None],
            Some(&main),
            Some(&off),
        );
        main.damage = 99;
        let view = Equipment::capture(&native);
        let equipment_api::Slot::Item(captured) = view.slot(equipment_api::EquipmentSlot::MainHand)
        else {
            panic!("captured main hand");
        };
        assert_eq!((captured.id, captured.damage), ("test:main", 7));
        assert!(matches!(
            view.slot(equipment_api::EquipmentSlot::Head),
            equipment_api::Slot::Empty
        ));
        let equipment_api::Slot::Item(captured) = view.slot(equipment_api::EquipmentSlot::OffHand)
        else {
            panic!("captured off hand");
        };
        assert_eq!((captured.id, captured.damage), ("test:off", 3));
    }
}
