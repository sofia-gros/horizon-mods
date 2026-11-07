//! One validated content/display pair reused for generation and registration.
//! This is owned declaration data, not an installed asset or registration receipt.
use super::{Definition, RenderDefinition, SlopeDistance};
use serde_json::Value;
use std::collections::BTreeMap;

/// A consumer-owned pair. Accessors are read-only; construction chooses no flow
/// defaults and performs no registration. Clone copies values, not world identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declaration {
    definition: Definition,
    render: RenderDefinition,
    slope_distance: Option<SlopeDistance>,
}

impl Declaration {
    /// Validate the exact same definition/render rules as explicit registration.
    /// Supplying default render facts is still an explicit style assertion.
    pub fn new(definition: Definition, render: RenderDefinition) -> Result<Self, String> {
        definition.validate()?;
        render.validate()?;
        Ok(Self { definition, render, slope_distance: None })
    }

    /// Opt in to bounded nearest-drop routing before the first registration.
    /// This cannot retrofit a live definition or change its other flow facts.
    pub fn with_slope_distance(mut self, distance: SlopeDistance) -> Self {
        self.slope_distance = Some(distance);
        self
    }

    pub fn slope_distance(&self) -> Option<SlopeDistance> {
        self.slope_distance
    }

    pub fn definition(&self) -> &Definition {
        &self.definition
    }

    pub fn render(&self) -> &RenderDefinition {
        &self.render
    }

    /// Generate the existing ordinary content and optional material template.
    /// Returns data only; the consumer still installs assets in its own package.
    pub fn content(&self) -> Result<BTreeMap<String, Value>, String> {
        super::content_with_render(&self.definition, &self.render)
    }

    /// Existing optional standalone-model selector, not a registered model.
    /// The consumer must declare it at the supported early preparation phase.
    pub fn render_material_model(&self) -> Result<Option<String>, String> {
        super::render_material_model(&self.definition, &self.render)
    }
}

/// Publish the entire group through the existing explicit-render transaction.
/// Identical pairs are idempotent; conflicts publish no new member. An empty
/// batch is a no-op. Existing legacy `register` semantics are not changed:
/// unlike a legacy repeat, an explicit default style conflicts with an already
/// registered custom style. Asset installation is outside this transaction.
pub fn register_declarations(
    declarations: impl IntoIterator<Item = Declaration>,
) -> Result<(), String> {
    let entries = declarations.into_iter().map(|entry| {
        // Keep registration validation identical even though the fields are private.
        entry.definition.validate()?;
        entry.render.validate()?;
        Ok((entry.definition, Some(entry.render), entry.slope_distance))
    }).collect::<Result<Vec<_>, String>>()?;
    if entries.is_empty() { return Ok(()); }
    let mut all = super::registry().write().map_err(|_| "fluid registry poisoned")?;
    super::register_entries_with_slope(&mut all, entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fluid::{self, RenderOrientation};

    fn definition(name: &str) -> Definition {
        Definition {
            id: format!("pluto_declaration_test:{name}"),
            block: format!("pluto_declaration_test:{name}_block"),
            bucket: format!("pluto_declaration_test:{name}_bucket"),
            texture: "pluto_declaration_test:block/still".into(),
            tick_delay: 7,
            drop_off: 2,
            light: 4,
        }
    }

    fn render() -> RenderDefinition {
        RenderDefinition {
            flowing_texture: "pluto_declaration_test:block/flow".into(),
            tint: 0x123456,
            orientation: RenderOrientation::Top,
        }
    }

    #[test]
    fn invalid_values_reject_construction_without_registration() {
        let mut bad = definition("invalid_definition");
        bad.tick_delay = 0;
        let id = bad.id.clone();
        assert!(Declaration::new(bad, render()).is_err());
        assert!(fluid::get(&id).is_none());
        let good = definition("invalid_render");
        let id = good.id.clone();
        let mut bad_render = render();
        bad_render.tint = 0x100_0000;
        assert!(Declaration::new(good, bad_render).is_err());
        assert!(fluid::get(&id).is_none());
    }

    #[test]
    fn generated_content_and_selector_use_the_exact_owned_pair() {
        let definition = definition("content");
        let render = render();
        let entry = Declaration::new(definition.clone(), render.clone()).unwrap();
        assert_eq!(entry.definition(), &definition);
        assert_eq!(entry.render(), &render);
        assert_eq!(entry.content(), fluid::content_with_render(&definition, &render));
        assert_eq!(entry.render_material_model(), fluid::render_material_model(&definition, &render));
        assert!(fluid::get(&definition.id).is_none());
    }

    #[test]
    fn default_pair_preserves_legacy_generated_content() {
        let definition = definition("default_content");
        let entry = Declaration::new(definition.clone(), RenderDefinition::default_for(&definition)).unwrap();
        assert_eq!(entry.content(), fluid::content(&definition));
        assert_eq!(entry.render_material_model().unwrap(), None);
    }

    #[test]
    fn real_registry_receives_both_pairs_and_identical_repeats() {
        let entries = ["batch_a", "batch_b"].map(|name| Declaration::new(definition(name), render()).unwrap());
        register_declarations(entries.clone()).unwrap();
        register_declarations(entries.clone()).unwrap();
        let snapshot = fluid::registry_snapshot().unwrap();
        for entry in entries {
            let id = &entry.definition().id;
            assert_eq!(snapshot.get(id), Some(entry.definition()));
            assert_eq!(snapshot.render(id), Some(entry.render()));
            assert_eq!(snapshot.for_block(&entry.definition().block), Some(entry.definition()));
        }
    }

    #[test]
    fn conflict_rolls_back_new_member_and_explicit_default_is_not_legacy_repeat() {
        let original = Declaration::new(definition("custom_existing"), render()).unwrap();
        register_declarations([original.clone()]).unwrap();
        // Legacy registration deliberately preserves the already registered style.
        fluid::register(original.definition().clone()).unwrap();
        let default = RenderDefinition::default_for(original.definition());
        let conflict = Declaration::new(original.definition().clone(), default).unwrap();
        let fresh = Declaration::new(definition("not_partially_published"), render()).unwrap();
        let id = fresh.definition().id.clone();
        assert!(register_declarations([fresh, conflict]).is_err());
        assert!(fluid::get(&id).is_none());
        let snapshot = fluid::registry_snapshot().unwrap();
        assert_eq!(snapshot.get(&original.definition().id), Some(original.definition()));
        assert_eq!(snapshot.render(&original.definition().id), Some(original.render()));
    }

    #[test]
    fn empty_group_is_successful() {
        register_declarations([]).unwrap();
    }
}
