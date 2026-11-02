// Island template piece and its persisted marker state.
use crate::nbt::{Compound, Tag};
use crate::worldgen::placement::PlacementQueryError;
use crate::worldgen::structure::{BoundingBox, Mirror, Rotation, StructureResources,
    legacy::{LegacyPiece, PieceContext, Result, TemplateHandler, TemplatePiece},
    processor::ProcessorList};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use super::{tconstruct_slime_island_marker_world as marker,
    tconstruct_slime_island_markers::TreeBudget, tconstruct_slime_tree_config::SlimeTreeSpec};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct MarkerConfig {
    pub structure: String,
    pub tree: Option<SlimeTreeSpec>,
    /// The selected original feature is retained even when its generator is
    /// unavailable, so saved partial starts do not lose the source choice.
    #[serde(default)]
    pub deferred_tree: Option<String>,
    pub grasses: Vec<(String, u32)>,
    pub vine: Option<String>,
}

#[derive(Debug)]
pub(crate) struct IslandPiece {
    template: TemplatePiece,
    markers: MarkerConfig,
    trees: Mutex<TreeBudget>,
}

impl IslandPiece {
    pub(crate) fn structure_name(&self) -> &str { &self.markers.structure }
    pub(crate) fn new(resources: &StructureResources, template_id: &str, pos: [i32; 3],
        rotation: Rotation, mirror: Mirror, markers: MarkerConfig, trees: u32) -> Option<Self> {
        let budget = TreeBudget::from_saved(trees)?;
        let template = resources.template(template_id)?;
        // StructureResources returns an empty template for missing files.
        // Source islands ignore entities; reject unexpected entities rather
        // than spawning them through the generic template path.
        if template.size.iter().any(|v| *v <= 0) || !template.entities.is_empty() { return None; }
        Some(Self { template: TemplatePiece::new("tconstruct:island_piece", template_id,
            template, pos, rotation, mirror, [0; 3], Arc::new(ProcessorList(Vec::new())),
            false, vec!["minecraft:structure_block".to_owned(), "minecraft:structure_void".to_owned()], TemplateHandler::Plain),
            markers, trees: Mutex::new(budget) })
    }

    pub(crate) fn from_saved(resources: &StructureResources, value: &Value) -> Option<Self> {
        let text = |key: &str| value.get(key)?.as_str();
        let integer = |key: &str| i32::try_from(value.get(key)?.as_i64()?).ok();
        let rotation = match text("Rot")? {
            "NONE" => Rotation::None, "CLOCKWISE_90" => Rotation::Clockwise90,
            "CLOCKWISE_180" => Rotation::Clockwise180,
            "COUNTERCLOCKWISE_90" => Rotation::Counterclockwise90, _ => return None,
        };
        let mirror = match text("Mi")? {
            "NONE" => Mirror::None, "LEFT_RIGHT" => Mirror::LeftRight,
            "FRONT_BACK" => Mirror::FrontBack, _ => return None,
        };
        let markers = serde_json::from_str(text("TCMarkers")?).ok()?;
        let trees = u32::try_from(integer("NumberOfTreesPlaced")?).ok()?;
        Self::new(resources, text("Template")?, [integer("TPX")?, integer("TPY")?, integer("TPZ")?],
            rotation, mirror, markers, trees)
    }
}

impl LegacyPiece for IslandPiece {
    fn bounding_box(&self) -> BoundingBox { self.template.bounding_box }
    fn type_name(&self) -> &'static str { "tconstruct:island_piece" }

    fn save(&self) -> Option<Compound> {
        let mut tag = self.template.saved();
        tag.insert("Mi".into(), Tag::String(self.template.mirror.saved_name().into()));
        tag.insert("Structure".into(), Tag::String(self.markers.structure.as_str().into()));
        let markers = serde_json::to_string(&self.markers).ok()?;
        tag.insert("TCMarkers".into(), Tag::String(markers.as_str().into()));
        tag.insert("NumberOfTreesPlaced".into(), Tag::Int(self.trees.lock().ok()?.placed() as i32));
        Some(tag)
    }

    fn post_process(&self, ctx: &mut PieceContext<'_>) -> Result<()> {
        // One lock protects source's per-piece count across chunk processing.
        let mut budget = self.trees.lock().map_err(|_| PlacementQueryError::Unavailable)?;
        marker::validate(ctx, self.markers.tree.as_ref().map(SlimeTreeSpec::config),
            &self.markers.grasses, self.markers.vine.as_deref())?;
        let Some(markers) = self.template.place_at(ctx, self.template.position, &[])? else { return Ok(()); };
        for (name, pos) in markers {
            match name.as_str() {
                "tconstruct:slime_tree" => {
                    if self.markers.deferred_tree.is_some() {
                        // Consume the source marker's initial draw, but do not
                        // replace an unsupported tree with another feature.
                        if budget.placed() < super::tconstruct_slime_island_markers::MAX_TREES { ctx.random.boolean(); }
                    } else {
                        marker::tree_marker(ctx, &mut budget, self.markers.tree.as_ref().map(SlimeTreeSpec::config), pos)?;
                    }
                }
                "tconstruct:slime_tall_grass" => { marker::grass_marker(ctx, &self.markers.grasses, pos)?; }
                "tconstruct:slime_vine" => { marker::vine_marker(ctx, self.markers.vine.as_deref(), pos)?; }
                _ => {},
            }
        }
        Ok(())
    }
}
